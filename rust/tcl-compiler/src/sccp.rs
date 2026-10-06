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
use crate::math_function_binding::ExpressionMathBindings;
use crate::ssa::{SsaFunction, SsaSourceView, SsaStatement, Symbol, ValueKey};
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
    /// Exact reached math dispatch dependencies consumed by successful folds.
    pub required_math_invocations: Vec<crate::command_binding::SourceMathInvocation>,
    /// Expression-entry dependencies retained independently of lazy reached calls.
    pub required_expression_preparations: Vec<crate::command_binding::SourceExpressionPreparation>,
    /// Per-SSA-value lattice entry.
    pub values: HashMap<ValueKey, LatticeValue>,
    /// Blocks reachable from `cfg.entry` under current assumptions.
    pub executable_blocks: HashSet<BlockId>,
    /// `(from_block, to_block)` edges known executable.
    pub executable_edges: HashSet<(BlockId, BlockId)>,
    /// Constant branches detected during propagation.
    pub constant_branches: Vec<ConstantBranch>,
}

/// Exact producer queried for an analysis value, separately from erasure proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExpressionEvaluationPoint {
    /// Whole expression of an SSA statement at its actual source point.
    Statement {
        /// CFG block containing the producer.
        block: BlockId,
        /// Statement index within that block.
        index: usize,
    },
    /// Whole expression of a CFG branch at its terminator.
    Branch {
        /// CFG block containing the condition.
        block: BlockId,
    },
}

/// Analysis value and obligations retained by the original expression producer.
/// This never inserts a constant into the execution lattice or licenses erasure.
#[derive(Debug, Clone, PartialEq)]
pub struct ExpressionAnalysis {
    /// Prepared-tree numeric interpretation with reached coercion/result dependencies.
    /// This does not discharge preparation or original argv/object evaluation;
    /// even an effect-free prepared tree cannot license deleting its producer.
    pub evaluation: crate::tcl_expr_eval::FoldEvaluation,
    /// Exact reached handlers consumed by the value calculation.
    pub required_math_invocations: Vec<crate::command_binding::SourceMathInvocation>,
    /// Whole-expression entry proof, even when every function call was skipped.
    pub required_expression_preparations: Vec<crate::command_binding::SourceExpressionPreparation>,
}

impl SccpResult {
    /// Query an original whole-expression producer using exact source/SSA evidence.
    /// Unknown native preparation, reads, handler identity or conflicting table
    /// owners decline. Reached conversion/result obligations remain in the result;
    /// numerical knowledge alone cannot justify deleting the producer. Input values
    /// come from the existing strict lattice, avoiding a second mutable value map.
    #[must_use]
    pub fn expression_analysis(
        &self,
        cfg: &CfgFunction,
        ssa: &SsaFunction,
        point: ExpressionEvaluationPoint,
        policy: FoldPolicy,
    ) -> Option<ExpressionAnalysis> {
        let input = analysis_expression_input(cfg, ssa, point)?;
        let bindings = ExpressionMathBindings::for_origin(
            &cfg.implicit_math_invocations,
            input.origin,
            input.base,
        )
        .with_preparations(&cfg.expression_preparations);
        let bindings = match point {
            ExpressionEvaluationPoint::Branch { block } => {
                materialised_branch_bindings(cfg, block, bindings)
            }
            ExpressionEvaluationPoint::Statement { .. } => bindings,
        };
        let environment = env_from_uses(input.uses, &self.values, input.source);
        let dependencies = MathDependencies {
            purpose: SolverPurpose::SemanticAnalysis,
            ..MathDependencies::default()
        };
        let _ = fold_math_expression(
            input.expression,
            &environment,
            policy,
            Some(MathFoldContext {
                point: Some(point),
                bindings,
                dependencies: Some(&dependencies),
                incoming_reads: None,
            }),
        );
        dependencies.analyses.into_inner().remove(&point)
    }
}

struct AnalysisExpressionInput<'a> {
    expression: &'a ExprNode,
    base: Option<u32>,
    uses: &'a HashMap<Symbol, crate::ssa::Version>,
    origin: Option<&'a crate::command_binding::ExecutedScriptSource>,
    source: SsaSourceView<'a>,
}

fn analysis_expression_input<'a>(
    cfg: &'a CfgFunction,
    ssa: &'a SsaFunction,
    point: ExpressionEvaluationPoint,
) -> Option<AnalysisExpressionInput<'a>> {
    match point {
        ExpressionEvaluationPoint::Statement { block, index } => {
            let statement = ssa.blocks.get(&block)?.statements.get(index)?;
            let (Statement::AssignExpr {
                expr, expr_base, ..
            }
            | Statement::ExprEval {
                expr, expr_base, ..
            }) = &statement.statement
            else {
                return None;
            };
            Some(AnalysisExpressionInput {
                expression: expr,
                base: *expr_base,
                uses: &statement.uses,
                origin: cfg
                    .statement_sources
                    .get(&(block, index))
                    .and_then(Option::as_deref),
                source: SsaSourceView::at_statement(ssa, block, index),
            })
        }
        ExpressionEvaluationPoint::Branch { block } => {
            let Some(Terminator::Branch {
                condition,
                condition_base,
                ..
            }) = cfg.blocks.get(&block)?.terminator.as_ref()
            else {
                return None;
            };
            Some(AnalysisExpressionInput {
                expression: condition,
                base: *condition_base,
                uses: &ssa.blocks.get(&block)?.exit_versions,
                origin: cfg
                    .terminator_sources
                    .get(&block)
                    .and_then(Option::as_deref),
                source: SsaSourceView::at_terminator(ssa, block),
            })
        }
    }
}

/// Inputs shared by the execution and semantic-value purposes of one solver.
#[derive(Clone, Copy)]
pub struct ValueFactInputs<'a> {
    /// Optional actual argument constants keyed by source parameter/version.
    pub param_constants: Option<&'a HashMap<(String, crate::ssa::Version), LatticeValue>>,
    /// Exact target semantics and original expression axes.
    pub policy: FoldPolicy,
    /// Compatibility exposure only; positioned reads retain temporal worlds.
    pub extra_escaping: &'a HashSet<String>,
    /// Actual registry and trace inventory.
    pub trace: TraceInputs<'a>,
    /// Optional known native substitution folding contracts.
    pub folds: Option<BuiltinFoldInputs<'a>>,
}

/// Immutable semantic-value projection. Known contents do not license erasure.
/// Producer obligations remain attached to their evaluation point, independently
/// of a later numeric use. Unknown selected-object bytes never become invented
/// native result strings. The same solver handles phis, stores and reachability
/// for this projection and the execution-safe compatibility result.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticValueFacts {
    result: SccpResult,
    expressions: HashMap<ExpressionEvaluationPoint, ExpressionAnalysis>,
    loops: HashMap<ExpressionEvaluationPoint, crate::static_loops::StaticLoopAnalysis>,
}

impl SemanticValueFacts {
    /// Actual known stored contents, rather than permission to replace a producer.
    #[must_use]
    pub fn contents(&self, key: ValueKey) -> Option<&LatticeValue> {
        self.result.values.get(&key)
    }

    /// Iterate immutable known contents for advisory consumers, never erasure.
    pub fn contents_iter(&self) -> impl Iterator<Item = (&ValueKey, &LatticeValue)> {
        self.result.values.iter()
    }

    /// Iterate producer numeric values with all required effects attached.
    pub fn expression_iter(
        &self,
    ) -> impl Iterator<Item = (&ExpressionEvaluationPoint, &ExpressionAnalysis)> {
        self.expressions.iter()
    }

    /// Numeric interpretation and retained effects of one original producer.
    #[must_use]
    pub fn expression(&self, point: ExpressionEvaluationPoint) -> Option<&ExpressionAnalysis> {
        self.expressions.get(&point)
    }

    /// Conditional-on-normal loop contents with every reached native operation retained.
    /// This does not prove completion without errors or permit erasing the loop.
    #[must_use]
    pub fn loop_analysis(
        &self,
        point: ExpressionEvaluationPoint,
    ) -> Option<&crate::static_loops::StaticLoopAnalysis> {
        self.loops.get(&point)
    }

    /// Whether semantic evaluation reaches this block. This does not license
    /// deleting the expression which selected its successor.
    #[must_use]
    pub fn reaches(&self, block: BlockId) -> bool {
        self.result.executable_blocks.contains(&block)
    }
}

/// Execution-erasure projection of the shared purpose-aware value solver.
#[must_use]
pub fn execution_value_facts(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    inputs: ValueFactInputs<'_>,
) -> SccpResult {
    solve_value_facts(cfg, ssa, inputs, SolverPurpose::ExecutionErasure).result
}

/// Compute semantic values without donating obligation-bearing constants to
/// legacy erasure consumers. Caches may retain this immutable purpose view;
/// preparation and source proof identities remain unchanged.
#[must_use]
pub fn semantic_value_facts(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    inputs: ValueFactInputs<'_>,
) -> SemanticValueFacts {
    let solved = solve_value_facts(cfg, ssa, inputs, SolverPurpose::SemanticAnalysis);
    SemanticValueFacts {
        result: solved.result,
        expressions: solved.expressions,
        loops: solved.loops,
    }
}

/// Borrowed diagnostic projection; values and reachability retain original producers.
/// This view is never an executable constant-replacement contract.
#[derive(Clone, Copy)]
pub(crate) struct DiagnosticValueFacts<'a> {
    result: &'a SccpResult,
}

impl<'a> DiagnosticValueFacts<'a> {
    pub(crate) fn from_semantic(facts: &'a SemanticValueFacts) -> Self {
        Self {
            result: &facts.result,
        }
    }

    pub(crate) fn compatibility(result: &'a SccpResult) -> Self {
        Self { result }
    }

    pub(crate) fn values(self) -> &'a HashMap<ValueKey, LatticeValue> {
        &self.result.values
    }

    pub(crate) fn executable_blocks(self) -> &'a HashSet<BlockId> {
        &self.result.executable_blocks
    }

    pub(crate) fn executable_edges(self) -> &'a HashSet<(BlockId, BlockId)> {
        &self.result.executable_edges
    }

    pub(crate) fn constant_branches(self) -> &'a [ConstantBranch] {
        &self.result.constant_branches
    }
}

/// Detached lazy cache for one immutable function's semantic-purpose view.
/// Equality compares complete retained inputs, never cache population.
#[derive(Debug, Default)]
pub struct SemanticValueProjection {
    inputs: Option<OwnedValueFactInputs>,
    cached: std::sync::OnceLock<SemanticValueFacts>,
}

#[derive(Debug, Clone, PartialEq)]
struct OwnedValueFactInputs {
    registry: tcl_registry::RegistrySnapshot,
    policy: FoldPolicy,
    param_constants: Option<HashMap<(String, crate::ssa::Version), LatticeValue>>,
    extra_escaping: HashSet<String>,
    traced_variables: BTreeSet<String>,
    has_dynamic_variable_trace: bool,
    mutations: Option<crate::command_binding::ModuleCommandMutations>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    defining_class: Option<String>,
    registry_engine: bool,
    trust: FoldTrust,
}

impl PartialEq for SemanticValueProjection {
    fn eq(&self, other: &Self) -> bool {
        self.inputs == other.inputs
    }
}

impl SemanticValueProjection {
    /// Metadata snapshot retained by this function's actual construction.
    /// Consumers still require their own positioned handler/source proof.
    pub(crate) fn retained_registry(&self) -> Option<&tcl_registry::CommandRegistry> {
        Some(self.inputs.as_ref()?.registry.registry())
    }

    /// Retain the exact construction inputs without running another solver.
    #[must_use]
    pub fn new(inputs: ValueFactInputs<'_>) -> Self {
        let folds = inputs.folds;
        Self {
            inputs: Some(OwnedValueFactInputs {
                registry: inputs.trace.registry.snapshot(),
                policy: inputs.policy,
                param_constants: inputs.param_constants.cloned(),
                extra_escaping: inputs.extra_escaping.clone(),
                traced_variables: inputs.trace.traced_variables.clone(),
                has_dynamic_variable_trace: inputs.trace.has_dynamic_variable_trace,
                mutations: folds.map(|folds| folds.mutations.clone()),
                dialect: folds.and_then(|folds| folds.dialect),
                defining_class: folds.and_then(|folds| folds.defining_class.map(str::to_owned)),
                registry_engine: folds.is_some_and(|folds| folds.registry_engine),
                trust: folds.map_or(FoldTrust::ObservedBindings, |folds| folds.trust),
            }),
            cached: std::sync::OnceLock::new(),
        }
    }

    /// Compute at most once for the containing immutable CFG/SSA proof graph.
    #[must_use]
    pub fn get(&self, cfg: &CfgFunction, ssa: &SsaFunction) -> Option<&SemanticValueFacts> {
        let input = self.inputs.as_ref()?;
        Some(self.cached.get_or_init(|| {
            let registry = input.registry.registry();
            semantic_value_facts(
                cfg,
                ssa,
                ValueFactInputs {
                    param_constants: input.param_constants.as_ref(),
                    policy: input.policy,
                    extra_escaping: &input.extra_escaping,
                    trace: TraceInputs {
                        registry,
                        traced_variables: &input.traced_variables,
                        has_dynamic_variable_trace: input.has_dynamic_variable_trace,
                    },
                    folds: input.mutations.as_ref().map(|mutations| BuiltinFoldInputs {
                        registry,
                        mutations,
                        dialect: input.dialect,
                        defining_class: input.defining_class.as_deref(),
                        registry_engine: input.registry_engine,
                        trust: input.trust,
                        proven_pure_parameters: false,
                    }),
                },
            )
        }))
    }

    /// Start a detached cache after proof relocation/restoration or rebasing.
    #[must_use]
    pub fn uncached(&self) -> Self {
        Self {
            inputs: self.inputs.clone(),
            cached: std::sync::OnceLock::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum SolverPurpose {
    #[default]
    ExecutionErasure,
    SemanticAnalysis,
}

struct SolvedValueFacts {
    result: SccpResult,
    expressions: HashMap<ExpressionEvaluationPoint, ExpressionAnalysis>,
    loops: HashMap<ExpressionEvaluationPoint, crate::static_loops::StaticLoopAnalysis>,
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
/// therefore what *every* builtin command-substitution fold in this module
/// is gated on, the per-command arms of [`try_fold_cmd_subst`] included.
/// A caller that passes `None` holds no whole-module view and gets no
/// builtin fold at all; `registry_engine` then selects whether a caller
/// that does hold one also gets the registry `const_fold` engine.
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
    /// Whether the registry `const_fold` engine may run in addition to the
    /// per-command arms in [`try_fold_cmd_subst`]. The shared per-unit
    /// lattice ([`crate::compilation_unit::FunctionUnit`]) supplies the
    /// trust fact with the engine off, which gates its per-command arms
    /// without widening what it folds; the optimiser's own re-run turns it
    /// on.
    pub registry_engine: bool,
    /// Which half of `mutations` gates the per-command arms — see
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
/// `Overdefined`, the same treatment [`crate::ssa::SsaSourceView::externally_mutable_by`] already gives
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
/// fact in hand, so builtin command substitutions fold at all — the
/// per-command arms of [`try_fold_cmd_subst`] always, and the registry
/// `const_fold` engine when [`BuiltinFoldInputs::registry_engine`] is set.
/// Passing `None` is byte-identical to [`sccp_with_extra_escaping`], which
/// folds no command substitution for want of that fact.
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
    execution_value_facts(
        cfg,
        ssa,
        ValueFactInputs {
            param_constants,
            policy,
            extra_escaping,
            trace,
            folds,
        },
    )
}

fn seed_value_facts(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    param_constants: Option<&HashMap<(String, crate::ssa::Version), LatticeValue>>,
    policy: FoldPolicy,
    registry: &CommandRegistry,
) -> (HashMap<ValueKey, LatticeValue>, tcl_dialect::LexerGrammar) {
    let mut values = HashMap::new();
    seed_parameter_constants(cfg, ssa, param_constants, registry, &mut values);
    let grammar = policy.preparation_context().map_or_else(
        || {
            registry
                .profile()
                .map_or_else(tcl_dialect::LexerGrammar::default, |profile| {
                    profile.grammar
                })
        },
        |context| context.lexer_grammar,
    );
    seed_live_in_roots(cfg, ssa, &mut values, grammar);
    (values, grammar)
}

fn solve_value_facts(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    inputs: ValueFactInputs<'_>,
    purpose: SolverPurpose,
) -> SolvedValueFacts {
    let ValueFactInputs {
        param_constants,
        policy,
        extra_escaping,
        trace,
        folds,
    } = inputs;
    let preds = compute_predecessors(cfg);
    let (mut values, grammar) = seed_value_facts(cfg, ssa, param_constants, policy, trace.registry);

    // Retained point contexts describe actual read and write observers and
    // contents origins. A later alias, opaque call or unreachable branch must
    // not retroactively erase an earlier definition. Carrierless compatibility
    // SSA still uses the conservative whole-function exposure summary.
    let escaping = compatibility_escaping(cfg, ssa, trace, extra_escaping);
    let required_math_invocations = MathDependencies::for_parameters(purpose, param_constants);
    let store_inputs = StoreInputs {
        math_dependencies: Some(&required_math_invocations),
        cfg: Some(cfg),
        escaping: &escaping,
        policy,
        has_dynamic_variable_trace: trace.has_dynamic_variable_trace
            && ssa.point_contexts.is_none(),
        folds,
        registry: trace.registry,
    };

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

                let incoming_exec =
                    incoming_executable_predecessors(*bn, &preds, &executable_edges);

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
                    StatementInputs {
                        store: &store_inputs,
                        clobbers: ssa.value_clobbers.get(bn),
                    },
                );

                // Terminator.
                let inputs = TerminatorInputs {
                    math_dependencies: Some(&required_math_invocations),
                    cfg,
                    ssa,
                    values: &values,
                    policy,
                    grammar,
                    registry: trace.registry,
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
            math_dependencies: Some(&required_math_invocations),
            policy,
            grammar,
            registry: trace.registry,
        },
    );

    required_math_invocations.finish(
        values,
        executable_blocks,
        executable_edges,
        constant_branches,
    )
}

fn incoming_executable_predecessors(
    block: BlockId,
    predecessors: &HashMap<BlockId, HashSet<BlockId>>,
    executable_edges: &HashSet<(BlockId, BlockId)>,
) -> Vec<BlockId> {
    predecessors
        .get(&block)
        .into_iter()
        .flatten()
        .copied()
        .filter(|predecessor| executable_edges.contains(&(*predecessor, block)))
        .collect()
}

fn compatibility_escaping(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    trace: TraceInputs<'_>,
    extra: &HashSet<String>,
) -> HashSet<String> {
    if ssa.point_contexts.is_some() {
        return HashSet::new();
    }
    let mut names = crate::var_observability::analyse_var_observability(cfg, trace.registry)
        .escaping_var_names();
    names.extend(extra.iter().cloned());
    names.extend(trace.traced_variables.iter().cloned());
    names
}

fn seed_parameter_constants(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    param_constants: Option<&HashMap<(String, crate::ssa::Version), LatticeValue>>,
    registry: &CommandRegistry,
    values: &mut HashMap<ValueKey, LatticeValue>,
) {
    if let Some(seed) = param_constants {
        // The interprocedural seed keys on the parameter *name* (a stable,
        // cache-safe identity); resolve each to this build's interned symbol.
        // A param never read in the body isn't interned, and its seed slot
        // would never be consulted, so dropping it is behaviour-neutral.
        for ((name, version), v) in seed {
            let symbol = ssa.point_contexts.as_ref().map_or_else(
                || ssa.var_symbol(name),
                |points| {
                    crate::var_resolve::canonical_variable_key(
                        name,
                        points.before_statement(cfg.entry, 0),
                        registry,
                    )
                    .and_then(|cell| ssa.cell_symbol(&cell))
                },
            );
            if let Some(symbol) = symbol {
                values.insert((symbol, *version), v.clone());
            }
        }
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
                let Some(sym) = ssa.var_symbol_at_terminator(*bn, &var) else {
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
    let block = ssa.block_id(&ssa_block.name).unwrap_or(BlockId(u32::MAX));
    let lookup = if ssa.point_contexts.is_some() {
        SsaSourceView::at_terminator(ssa, block)
    } else {
        SsaSourceView::unpositioned(ssa)
    };
    let mut any_operand = false;
    let mut any_unknown = false;
    for name in crate::var_refs::vars_in_expr(condition, grammar) {
        let (sym, ver) = if lookup.is_positioned() {
            let Some(read) = lookup.read_spelling(&name) else {
                return false;
            };
            let Some(version) = read.version else {
                return false;
            };
            (read.symbol, version)
        } else {
            let Some(symbol) = lookup.symbol(&name) else {
                continue;
            };
            (
                symbol,
                ssa_block.exit_versions.get(&symbol).copied().unwrap_or(0),
            )
        };
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

#[derive(Clone, Copy)]
struct StatementInputs<'a> {
    store: &'a StoreInputs<'a>,
    clobbers: Option<&'a crate::ssa::BlockValueClobbers>,
}

/// Evaluate each statement's defs for one block, widening across barriers.
/// Returns `true` if any lattice value changed. Extracted from [`sccp`].
/// Store proof inputs shared by every SCCP entry point.
struct StoreInputs<'a> {
    math_dependencies: Option<&'a MathDependencies>,
    cfg: Option<&'a CfgFunction>,
    escaping: &'a HashSet<String>,
    policy: FoldPolicy,
    has_dynamic_variable_trace: bool,
    folds: Option<BuiltinFoldInputs<'a>>,
    registry: &'a CommandRegistry,
}

fn observed_store_keys(
    ssa: &SsaFunction,
    block: BlockId,
    index: usize,
    statement: &Statement,
    registry: &CommandRegistry,
) -> HashSet<crate::var_resolve::VariableCellKey> {
    let Some(points) = &ssa.point_contexts else {
        return HashSet::new();
    };
    crate::place_bridge::def_places_with_continuation(
        statement,
        points.before_statement(block, index),
        points.after_statement(block, index),
        registry,
    )
    .into_iter()
    .filter(|place| place.observed)
    .filter_map(|place| crate::var_resolve::canonical_binding_value_key(&place))
    .collect()
}

fn statement_math_context<'a>(
    cfg: Option<&'a CfgFunction>,
    block: BlockId,
    index: usize,
    statement: &Statement,
    dependencies: Option<&'a MathDependencies>,
) -> Option<MathFoldContext<'a>> {
    let cfg = cfg?;
    let base = match statement {
        Statement::AssignExpr { expr_base, .. } | Statement::ExprEval { expr_base, .. } => {
            *expr_base
        }
        _ => None,
    };
    Some(MathFoldContext {
        point: Some(ExpressionEvaluationPoint::Statement { block, index }),
        dependencies,
        incoming_reads: None,
        bindings: ExpressionMathBindings::for_origin(
            &cfg.implicit_math_invocations,
            cfg.statement_sources
                .get(&(block, index))
                .and_then(Option::as_deref),
            base,
        )
        .with_preparations(&cfg.expression_preparations),
    })
}

/// Widen only fresh scalar keys; original stores and read versions remain intact.
fn widen_registry_clobbers(
    values: &mut HashMap<ValueKey, LatticeValue>,
    clobbers: Option<&crate::ssa::BlockValueClobbers>,
    index: usize,
    folds: Option<BuiltinFoldInputs<'_>>,
) -> bool {
    let mut changed = false;
    for (&var, &(prior, fresh)) in clobbers
        .and_then(|markers| markers.get(&index))
        .into_iter()
        .flatten()
    {
        let value = if folds.is_some_and(|f| f.proven_pure_parameters)
            && matches!(values.get(&(var, 0)), Some(LatticeValue::Const(_)))
        {
            values
                .get(&(var, prior))
                .cloned()
                .unwrap_or(LatticeValue::Unknown)
        } else {
            LatticeValue::Overdefined
        };
        changed |= set_value(values, (var, fresh), &value);
    }
    changed
}

fn sccp_process_statements(
    values: &mut HashMap<ValueKey, LatticeValue>,
    ssa_block: &crate::ssa::SsaBlock,
    ssa: &SsaFunction,
    inputs: StatementInputs<'_>,
) -> bool {
    let StatementInputs { store, clobbers } = inputs;
    let StoreInputs {
        math_dependencies,
        cfg,
        escaping,
        policy,
        has_dynamic_variable_trace,
        folds,
        registry,
    } = *store;
    let mut changed = false;
    let block = ssa.block_id(&ssa_block.name).unwrap_or(BlockId(u32::MAX));
    for (index, stmt_ssa) in ssa_block.statements.iter().enumerate() {
        // Statement defs become live after its inputs and barrier effect.
        let registry_barrier = stmt_ssa.statement.synthetic_marker()
            == Some(crate::ir::SyntheticMarker::RegistryBarrier);
        if registry_barrier {
            changed |= widen_registry_clobbers(values, clobbers, index, folds);
            continue;
        }
        let lookup = if ssa.point_contexts.is_some() {
            SsaSourceView::at_statement(ssa, block, index)
        } else {
            SsaSourceView::unpositioned(ssa)
        };
        if matches!(
            stmt_ssa.statement,
            Statement::Barrier { .. } | Statement::NativeCall { .. } | Statement::UpFrame { .. }
        ) {
            // Executable barriers widen tracked values except version-0
            // parameter seeds. Registry boundaries use fresh versions above,
            // preserving every earlier proof. Ordinary barrier seeds hold the caller's
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
            let keys: Vec<ValueKey> = if ssa.point_contexts.is_none() {
                values.keys().copied().collect()
            } else {
                // Exact reads after an unrepresented store have no contents
                // version. Earlier SSA versions remain immutable evidence.
                Vec::new()
            };
            for k in keys {
                if k.1 != 0 && set_value(values, k, &LatticeValue::Overdefined) {
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
        let observed = observed_store_keys(ssa, block, index, &stmt_ssa.statement, registry);
        let math =
            statement_math_context(cfg, block, index, &stmt_ssa.statement, math_dependencies)
                .map(|context| context.with_incoming_reads(lookup, registry));
        if math_dependencies.is_some_and(|ledger| ledger.purpose == SolverPurpose::SemanticAnalysis)
            && let Statement::ExprEval { expr, .. } = &stmt_ssa.statement
        {
            let environment = env_from_uses(&stmt_ssa.uses, values, lookup);
            let _ = fold_math_expression(expr, &environment, policy, math);
        }
        for (&var, ver) in &stmt_ssa.defs {
            let val = if observed.contains(ssa.cell_key(var))
                || lookup.externally_mutable_by(var, escaping, has_dynamic_variable_trace, registry)
                    != Some(false)
                || ssa.is_array_root_refresh_version(var, *ver)
                || stmt_ssa.destruction_defs.contains(&var)
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
                            &evaluate_def_with_math(
                                stmt_ssa, &*values, lookup, policy, folds, math,
                            ),
                        )
                    }
                    None => LatticeValue::Overdefined,
                }
            } else {
                evaluate_def_with_math(stmt_ssa, &*values, lookup, policy, folds, math)
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
    math_dependencies: Option<&'a MathDependencies>,
    registry: &'a CommandRegistry,
    cfg: &'a CfgFunction,
    ssa: &'a SsaFunction,
    values: &'a HashMap<ValueKey, LatticeValue>,
    policy: FoldPolicy,
    /// The document's lexer grammar — the branch condition's `Raw` operand
    /// texts are re-read under it when their variables are collected.
    grammar: tcl_dialect::LexerGrammar,
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
        math_dependencies,
        cfg,
        ssa,
        values,
        policy,
        grammar,
        registry,
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
            let decision = branch_decision(
                cfg,
                ssa,
                bn,
                ssa_block,
                condition,
                values,
                BranchFold {
                    math_dependencies,
                    registry,
                    policy,
                    grammar,
                },
            );
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
        Terminator::Return { .. } | Terminator::Complete { .. } => {}
    }
    // `try` exception edges sourced at `bn`: when `bn` is executable the
    // handler is reachable (a throw can occur in the body).
    for (from, to) in cfg.exception_edges.iter().chain(&cfg.analysis_edges) {
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
        let decision = branch_decision(cfg, ssa, *bn, ssa_block, condition, values, fold);
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

/// Existence folds from exact invocation and contents-presence proofs.
/// Both optimiser rewrites and diagnostics consume this positioned entry point.
/// Missing nested dispatch or uncertain presence declines without a name scan.
#[must_use]
pub fn existence_constant_branches_with_ssa(
    cfg: &CfgFunction,
    frame: ExistenceFrame<'_>,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    ssa: &SsaFunction,
) -> Vec<ConstantBranch> {
    let Some(points) = &ssa.point_contexts else {
        return existence_constant_branches(
            cfg,
            frame,
            registry,
            crate::dynamic_names::dynamic_name_barrier(cfg, registry, config),
            config,
        );
    };
    cfg.blocks
        .iter()
        .filter_map(|(&id, block)| {
            let Terminator::Branch {
                condition,
                true_target,
                false_target,
                span: Some(span),
                condition_base: Some(base),
            } = block.terminator.as_ref()?
            else {
                return None;
            };
            let tokens = points.source_tokens_at(id, usize::MAX)?;
            let (query, context) =
                crate::existence_query::in_expr_at(condition, *base, tokens, registry, config)?;
            let place =
                crate::var_resolve::resolve_literal_place(&query.var, &context, false, registry);
            let exists = match context.contents_presence(&place) {
                crate::var_resolve::ContentsPresence::Undefined => false,
                crate::var_resolve::ContentsPresence::Defined => match query.kind {
                    crate::existence_query::ExistenceKind::AnyVariable => true,
                    crate::existence_query::ExistenceKind::Array => {
                        // Literal scalar contents and untouched own formal cells
                        // prove scalar storage. Other defined cells may be arrays.
                        let formal = frame.params.contains(&query.var)
                            && context.contents_origin(&place)
                                == crate::var_resolve::ContentsOrigin::Incoming
                            && place.cell.as_ref().is_some_and(|cell| {
                                matches!(
                                    &cell.owner, crate::place::CellOwner::Activation(owner)
                                        if context.activation.as_ref() == Some(owner)
                                )
                            });
                        if !formal && context.literal_contents_at(&place, registry).is_none() {
                            return None;
                        }
                        false
                    }
                },
                crate::var_resolve::ContentsPresence::Unknown
                | crate::var_resolve::ContentsPresence::DefinedOrUndefined => return None,
            };
            let value = exists ^ query.negated;
            let (taken, not_taken) = if value {
                (*true_target, *false_target)
            } else {
                (*false_target, *true_target)
            };
            Some(ConstantBranch {
                block: block.name.clone(),
                span: Some(*span),
                condition: crate::expr_ast::expr_text(condition),
                value,
                taken_target: cfg.block_name(taken).to_owned(),
                not_taken_target: cfg.block_name(not_taken).to_owned(),
            })
        })
        .collect()
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
        b.statements.iter().any(|s| {
            matches!(
                s,
                Statement::Barrier { .. }
                    | Statement::NativeCall { .. }
                    | Statement::UpFrame { .. }
            )
        })
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
///
/// Holds no whole-module command view, so it folds **no** builtin command
/// substitution: there is no evidence that `[llength …]` still means the
/// builtin rather than a user `proc` of that name. A caller with the fact
/// uses [`evaluate_def_with_folds`].
#[must_use]
pub fn evaluate_def<'a, S: std::hash::BuildHasher>(
    stmt_ssa: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    ssa: impl Into<SsaSourceView<'a>>,
    policy: FoldPolicy,
) -> LatticeValue {
    let ssa = ssa.into();
    evaluate_def_with_folds(stmt_ssa, values, ssa, policy, None)
}

/// [`evaluate_def`] with an optional registry builtin-fold context: when
/// `folds` is supplied, an `AssignValue` command-substitution
/// RHS additionally consults the registry `const_fold` engine — see
/// [`BuiltinFoldInputs`]. Pass a [`SsaSourceView::at_statement`] for production
/// evaluation so source aliases resolve at this operation. A borrowed SSA
/// function retains only the unique whole-function compatibility projection.
/// `None` is byte-identical to [`evaluate_def`].
#[must_use]
pub fn evaluate_def_with_folds<'a, S: std::hash::BuildHasher>(
    stmt_ssa: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    ssa: impl Into<SsaSourceView<'a>>,
    policy: FoldPolicy,
    folds: Option<BuiltinFoldInputs<'_>>,
) -> LatticeValue {
    evaluate_def_with_math(stmt_ssa, values, ssa.into(), policy, folds, None)
}

#[derive(Default)]
struct MathDependencies {
    purpose: SolverPurpose,
    incoming_parameters: HashMap<String, ConstValue>,
    analyses: std::cell::RefCell<HashMap<ExpressionEvaluationPoint, ExpressionAnalysis>>,
    loops: std::cell::RefCell<
        HashMap<ExpressionEvaluationPoint, crate::static_loops::StaticLoopAnalysis>,
    >,
    invocations: std::cell::RefCell<Vec<crate::command_binding::SourceMathInvocation>>,
    preparations: std::cell::RefCell<Vec<crate::command_binding::SourceExpressionPreparation>>,
}

impl MathDependencies {
    fn for_parameters(
        purpose: SolverPurpose,
        parameters: Option<&HashMap<(String, crate::ssa::Version), LatticeValue>>,
    ) -> Self {
        if purpose == SolverPurpose::ExecutionErasure {
            return Self::default();
        }
        Self {
            purpose,
            incoming_parameters: parameters
                .into_iter()
                .flat_map(|parameters| parameters.iter())
                .filter_map(|((name, version), value)| match value {
                    LatticeValue::Const(value) if *version == 0 => {
                        Some((name.clone(), value.clone()))
                    }
                    _ => None,
                })
                .collect(),
            ..Self::default()
        }
    }

    fn finish(
        self,
        values: HashMap<ValueKey, LatticeValue>,
        executable_blocks: HashSet<BlockId>,
        executable_edges: HashSet<(BlockId, BlockId)>,
        constant_branches: Vec<ConstantBranch>,
    ) -> SolvedValueFacts {
        SolvedValueFacts {
            expressions: self.analyses.into_inner(),
            loops: self.loops.into_inner(),
            result: SccpResult {
                required_math_invocations: self.invocations.into_inner(),
                required_expression_preparations: self.preparations.into_inner(),
                values,
                executable_blocks,
                executable_edges,
                constant_branches,
            },
        }
    }

    fn retain(
        &self,
        invocations: &[crate::command_binding::SourceMathInvocation],
        preparations: &[crate::command_binding::SourceExpressionPreparation],
    ) -> bool {
        let old_invocations = self.invocations.borrow();
        let old_preparations = self.preparations.borrow();
        let mut required = None;
        for prerequisite in old_invocations
            .iter()
            .chain(invocations)
            .filter_map(|proof| proof.fixed_prerequisite())
            .chain(
                old_preparations
                    .iter()
                    .chain(preparations)
                    .filter_map(|proof| proof.witness.fixed_functions()),
            )
        {
            if !crate::math_function_binding::native_math_prerequisites_compatible(
                required,
                Some(prerequisite),
            ) {
                return false;
            }
            required = Some(prerequisite);
        }
        drop(old_invocations);
        drop(old_preparations);
        let mut retained_invocations = self.invocations.borrow_mut();
        for proof in invocations {
            if !retained_invocations.contains(proof) {
                retained_invocations.push(proof.clone());
            }
        }
        let mut retained_preparations = self.preparations.borrow_mut();
        for proof in preparations {
            if !retained_preparations.contains(proof) {
                retained_preparations.push(proof.clone());
            }
        }
        true
    }
}

#[derive(Clone, Copy)]
struct MathFoldContext<'a> {
    point: Option<ExpressionEvaluationPoint>,
    bindings: ExpressionMathBindings<'a>,
    dependencies: Option<&'a MathDependencies>,
    incoming_reads: Option<(SsaSourceView<'a>, &'a CommandRegistry)>,
}

impl<'a> MathFoldContext<'a> {
    fn with_incoming_reads(self, source: SsaSourceView<'a>, registry: &'a CommandRegistry) -> Self {
        Self {
            incoming_reads: Some((source, registry)),
            ..self
        }
    }
}

fn math_dependencies_compatible(
    previous: &crate::command_binding::SourceMathInvocation,
    current: &crate::command_binding::SourceMathInvocation,
) -> bool {
    crate::math_function_binding::native_math_prerequisites_compatible(
        previous.fixed_prerequisite(),
        current.fixed_prerequisite(),
    )
}

enum FoldedExpressionContents {
    Numeric(TclValue),
    Bytes(String),
}

/// Logical caller inputs are contents facts across proved ordinary incoming
/// slots. They do not allocate an SSA version or prove a native object state.
fn semantic_incoming_environment(
    expression: &ExprNode,
    base: Option<u32>,
    environment: &Env,
    context: Option<MathFoldContext<'_>>,
) -> Option<Env> {
    let context = context?;
    let ledger = context.dependencies?;
    if ledger.purpose != SolverPurpose::SemanticAnalysis || ledger.incoming_parameters.is_empty() {
        return None;
    }
    let (source, registry) = context.incoming_reads?;
    let mut admitted = HashMap::<&str, bool>::new();
    let mut pending = vec![expression];
    while let Some(node) = pending.pop() {
        match node {
            ExprNode::Var { name, .. } if ledger.incoming_parameters.contains_key(name) => {
                let proved = source
                    .read_expression_incoming_slot(node, base, name, registry)
                    .is_some();
                *admitted.entry(name).or_insert(true) &= proved;
            }
            ExprNode::Binary { left, right, .. } => pending.extend([left.as_ref(), right.as_ref()]),
            ExprNode::Unary { operand, .. } => pending.push(operand),
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                pending.extend([
                    condition.as_ref(),
                    true_branch.as_ref(),
                    false_branch.as_ref(),
                ]);
            }
            ExprNode::Call { args, .. } => pending.extend(args),
            _ => {}
        }
    }
    let mut result = environment.clone();
    for (name, _) in admitted.into_iter().filter(|(_, proved)| *proved) {
        if let Some(value) = ledger.incoming_parameters.get(name) {
            result
                .entry(name.to_owned())
                .or_insert_with(|| const_to_env_value(value));
        }
    }
    Some(result)
}

impl FoldedExpressionContents {
    fn into_const(self) -> ConstValue {
        match self {
            Self::Numeric(value) => tcl_value_to_const(value),
            Self::Bytes(value) => ConstValue::String(value),
        }
    }
}

fn retained_result_needs_native_protocol(
    preparation: Option<&crate::command_binding::SourceExpressionPreparation>,
    point: Option<ExpressionEvaluationPoint>,
    analysis: bool,
) -> bool {
    if analysis || matches!(point, Some(ExpressionEvaluationPoint::Branch { .. })) {
        return false;
    }
    // ConstValue retains contents only. A retained result instruction also
    // carries normalisation, allocation or pool-reuse semantics that this
    // projection cannot preserve. Consumed branch values have no output object.
    preparation.is_some_and(|preparation| {
        crate::expression_rewrite::expression_result_protocol_equivalence(
            &preparation.witness,
            None,
        )
        .is_err()
    })
}

fn fold_math_expression(
    expression: &ExprNode,
    environment: &Env,
    policy: FoldPolicy,
    context: Option<MathFoldContext<'_>>,
) -> Option<FoldedExpressionContents> {
    let ledger = context.and_then(|context| context.dependencies);
    let point = context.and_then(|context| context.point);
    let analysis_purpose =
        ledger.is_some_and(|ledger| ledger.purpose == SolverPurpose::SemanticAnalysis);
    if let (true, Some(ledger), Some(point)) = (analysis_purpose, ledger, point) {
        ledger.analyses.borrow_mut().remove(&point);
    }
    let preparation = context.and_then(|context| {
        context
            .bindings
            .preparation_for_context(&policy.preparation_context()?)
    });
    if context.is_some_and(|context| context.bindings.is_positioned()) && preparation.is_none()
        || retained_result_needs_native_protocol(preparation, point, analysis_purpose)
    {
        return None;
    }
    let expression = preparation.map_or(expression, |preparation| preparation.witness.tree());
    let incoming_environment = semantic_incoming_environment(
        expression,
        preparation.map(|preparation| preparation.source.base()),
        environment,
        context,
    );
    let environment = incoming_environment.as_ref().unwrap_or(environment);
    let numeric_operands = context.and_then(|context| {
        let (source, registry) = context.incoming_reads?;
        crate::native_numeric::expression_operands(expression, |node| {
            source.read_expression_native_numeric(
                node,
                preparation.map(|proof| proof.source.base()),
                registry,
            )
        })
    });
    let mut numeric_environment = environment.clone();
    if let Some((native, _)) = &numeric_operands {
        numeric_environment.extend(native.clone());
    }
    let environment = &numeric_environment;
    let consumed = std::cell::RefCell::new(Vec::new());
    let query = |function: &str, start| {
        let context = context?;
        let call = context.bindings.resolved_call(function, start)?;
        let proof = call.invocation;
        crate::math_function_binding::native_fold_dependency(proof)?;
        if consumed
            .borrow()
            .iter()
            .any(|other| !math_dependencies_compatible(other, proof))
            || ledger.is_some_and(|ledger| {
                ledger
                    .invocations
                    .borrow()
                    .iter()
                    .any(|other| !math_dependencies_compatible(other, proof))
            })
        {
            return None;
        }
        consumed.borrow_mut().push(proof.clone());
        Some(call.target())
    };
    let (value, analysis) = evaluate_prepared_numeric_expression(
        expression,
        environment,
        policy,
        &query,
        numeric_operands.as_ref().map(|(_, proofs)| proofs),
        analysis_purpose,
        matches!(point, Some(ExpressionEvaluationPoint::Branch { .. })),
    )?;
    let consumed = consumed.into_inner();
    if let Some(ledger) = ledger {
        let preparations: &[crate::command_binding::SourceExpressionPreparation] =
            preparation.map_or(&[], std::slice::from_ref);
        if !ledger.retain(&consumed, preparations) {
            return None;
        }
        if let (Some(point), Some(evaluation)) = (point, analysis) {
            ledger.analyses.borrow_mut().insert(
                point,
                ExpressionAnalysis {
                    evaluation,
                    required_math_invocations: consumed,
                    required_expression_preparations: preparations.to_vec(),
                },
            );
        }
    }
    value
}

fn evaluate_prepared_numeric_expression(
    expression: &ExprNode,
    environment: &Env,
    policy: FoldPolicy,
    query: &crate::tcl_expr_eval::ResolvedMathQuery<'_>,
    operands: Option<&crate::tcl_expr_eval::NativeOperandProofs>,
    analysis_purpose: bool,
    branch_result: bool,
) -> Option<(
    Option<FoldedExpressionContents>,
    Option<crate::tcl_expr_eval::FoldEvaluation>,
)> {
    if analysis_purpose {
        let evaluation = crate::tcl_expr_eval::analyse_tcl_expr_with_resolved_math_bindings(
            expression,
            environment,
            policy,
            query,
            operands,
        )?;
        let value = if branch_result {
            Some(FoldedExpressionContents::Numeric(evaluation.value.clone()))
        } else {
            analysis_native_contents(&evaluation)
        };
        Some((value, Some(evaluation)))
    } else {
        // Operand receipts close input conversion, not the arithmetic result's
        // allocation or sharing with a substituted literal. Branch truth is
        // consumed internally; represented stores and returns retain the
        // original numeric result operation until its output protocol is proved.
        if !branch_result && operands.is_some_and(|operands| !operands.is_empty()) {
            return None;
        }
        Some((
            Some(FoldedExpressionContents::Numeric(
                crate::tcl_expr_eval::eval_tcl_expr_with_proved_operands(
                    expression,
                    environment,
                    policy,
                    query,
                    operands.unwrap_or(&crate::tcl_expr_eval::NativeOperandProofs::new()),
                )?,
            )),
            None,
        ))
    }
}

fn analysis_native_contents(
    evaluation: &crate::tcl_expr_eval::FoldEvaluation,
) -> Option<FoldedExpressionContents> {
    use crate::tcl_expr_eval::NativeExpressionResultDependency;
    match &evaluation.result_dependency {
        Some(NativeExpressionResultDependency::SelectedOperand { existing_bytes, .. }) => {
            existing_bytes
                .as_ref()
                .map(|bytes| FoldedExpressionContents::Bytes(bytes.clone()))
        }
        Some(NativeExpressionResultDependency::StringResult { bytes }) => {
            Some(FoldedExpressionContents::Bytes(bytes.clone()))
        }
        None => Some(FoldedExpressionContents::Numeric(evaluation.value.clone())),
    }
}

fn evaluate_def_with_math<S: std::hash::BuildHasher>(
    stmt_ssa: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    ssa: SsaSourceView<'_>,
    policy: FoldPolicy,
    folds: Option<BuiltinFoldInputs<'_>>,
    math: Option<MathFoldContext<'_>>,
) -> LatticeValue {
    match &stmt_ssa.statement {
        Statement::AssignConst { value, .. } => LatticeValue::Const(parse_literal_value(value)),
        Statement::AssignExpr { expr, .. } => {
            let env = env_from_uses(&stmt_ssa.uses, values, ssa);
            fold_math_expression(expr, &env, policy, math)
                .map_or(LatticeValue::Overdefined, |value| {
                    LatticeValue::Const(value.into_const())
                })
        }
        Statement::AssignValue { value, .. } => {
            // Fold when the RHS is either a plain literal
            // (no command substitution), a simple `$var` that
            // resolves to a lattice Const, or a `[cmd args...]`
            // that try_fold_cmd_subst (or, under `folds`, the registry
            // const-fold engine) recognises.
            fold_retained_assignment_expression(stmt_ssa, values, ssa, policy, folds, math)
                .unwrap_or_else(|| {
                    fold_assign_value(value, &stmt_ssa.uses, values, ssa, policy, folds)
                })
        }
        Statement::Call {
            command,
            args,
            defs,
            tokens,
            ..
        } if single_iteration_binding(command, args, defs, tokens.as_ref()) => {
            fold_iteration_value(&args[0], stmt_ssa, values, ssa, policy, folds)
        }
        Statement::Call {
            tokens: Some(tokens),
            ..
        } => fold_normal_store_value(tokens, stmt_ssa, values, ssa, policy, folds, math)
            .unwrap_or(LatticeValue::Overdefined),
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
            let Some(sym) = ssa.symbol(crate::naming::element_var_name(name)) else {
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

#[derive(Clone, Copy)]
struct RetainedExpressionFoldInputs<'a> {
    registry: &'a CommandRegistry,
    policy: FoldPolicy,
    math: Option<MathFoldContext<'a>>,
}

fn fold_retained_assignment_expression<S: std::hash::BuildHasher>(
    statement: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    source: SsaSourceView<'_>,
    policy: FoldPolicy,
    folds: Option<BuiltinFoldInputs<'_>>,
    math: Option<MathFoldContext<'_>>,
) -> Option<LatticeValue> {
    let registry = folds?.registry;
    let tokens = statement.statement.tokens()?;
    let invocation = crate::registry_invocation::normal_transfer_invocation(
        registry,
        registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile),
        tokens,
    )?;
    let (_, word) = invocation
        .stored_value_operand(&tokens.source_binding.as_ref()?.variable_context, registry)?;
    fold_retained_expression_word(
        word,
        tokens,
        statement,
        values,
        source,
        RetainedExpressionFoldInputs {
            registry,
            policy,
            math,
        },
    )
}

fn fold_retained_expression_word<S: std::hash::BuildHasher>(
    word: &crate::ir::WordExpr,
    tokens: &crate::ir::CommandTokens,
    statement: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    source: SsaSourceView<'_>,
    inputs: RetainedExpressionFoldInputs<'_>,
) -> Option<LatticeValue> {
    let (spelling, site) = word.sole_command_substitution()?;
    if inputs
        .math?
        .dependencies
        .is_some_and(|ledger| ledger.purpose == SolverPurpose::SemanticAnalysis)
        && let Some(value) = fold_prepared_invocation_expression(spelling, site, inputs)
    {
        return Some(value);
    }
    let expressions = crate::word_subst::lifted_source_expressions(Some(tokens), inputs.registry);
    let mut matching = expressions
        .iter()
        .filter(|expression| expression.span == site.span);
    let expression = matching.next()?;
    if matching.next().is_some() {
        return None;
    }
    let context = inputs.math?;
    let context = MathFoldContext {
        bindings: context.bindings.at_base(expression.expression_base),
        ..context
    };
    let environment = env_from_uses(&statement.uses, values, source);
    fold_math_expression(
        &expression.expression,
        &environment,
        inputs.policy,
        Some(context),
    )
    .map_or(Some(LatticeValue::Overdefined), |value| {
        Some(LatticeValue::Const(value.into_const()))
    })
}

fn fold_prepared_invocation_expression(
    spelling: &str,
    site: &crate::ir::SourceSite,
    inputs: RetainedExpressionFoldInputs<'_>,
) -> Option<LatticeValue> {
    let grammar = inputs.policy.preparation_context()?.lexer_grammar;
    let invocation = crate::word_subst::nested_command_words(
        spelling,
        site,
        tcl_lexer::LexerConfig::from_grammar(grammar),
    )
    .ok()?;
    let offset = invocation.argv.first()?.start();
    let context = inputs.math?;
    let context = MathFoldContext {
        bindings: context.bindings.at_invocation(offset)?,
        ..context
    };
    let preparation = context
        .bindings
        .preparation_for_context(&inputs.policy.preparation_context()?)?;
    fold_math_expression(
        preparation.witness.tree(),
        &Env::default(),
        inputs.policy,
        Some(context),
    )
    .map(|value| LatticeValue::Const(value.into_const()))
}

fn fold_iteration_value<S: std::hash::BuildHasher>(
    argument: &str,
    statement: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    source: SsaSourceView<'_>,
    policy: FoldPolicy,
    folds: Option<BuiltinFoldInputs<'_>>,
) -> LatticeValue {
    // `foreach v LIST` / `lmap v LIST` folds the
    // iteration variable to the CONSTSET of elements when
    // LIST is a literal, resolves to a Const(String)
    // through the lattice, or is a command substitution
    // (`[list a b c]`, `[format …]`) that folds to a
    // constant list. Multi-variable and multi-list
    // foreaches are left as Overdefined.
    let elements = extract_foreach_elements(argument, policy.word_rules)
        .or_else(|| {
            resolve_foreach_list_via_lattice(
                argument,
                &statement.uses,
                values,
                source,
                policy.word_rules,
            )
        })
        .or_else(|| {
            // `foreach v [list a b c]` — fold the command substitution
            // to a constant list string, then split into elements.
            let arg = argument.trim();
            if arg.starts_with('[')
                && arg.ends_with(']')
                && let Some(LatticeValue::Const(ConstValue::String(s))) =
                    try_fold_cmd_subst(arg, &statement.uses, values, source, policy, folds)
            {
                return Some(split_list_values(&s, policy.word_rules));
            }
            None
        });
    match elements {
        Some(items) if items.is_empty() => LatticeValue::Overdefined,
        Some(items) => {
            let consts: Vec<ConstValue> = items.iter().map(|s| parse_literal_value(s)).collect();
            if consts.len() == 1 {
                LatticeValue::Const(consts.into_iter().next().unwrap())
            } else {
                LatticeValue::constset(consts)
            }
        }
        None => LatticeValue::Overdefined,
    }
}

/// A normal handler's bounded store is separate from its compiler completion.
/// Frozen argv values take precedence; original variable reads retain their
/// exact temporal SSA dependency. This never licenses executing/eliding a call.
fn fold_normal_store_value<S: std::hash::BuildHasher>(
    tokens: &crate::ir::CommandTokens,
    statement: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    source: SsaSourceView<'_>,
    policy: FoldPolicy,
    folds: Option<BuiltinFoldInputs<'_>>,
    math: Option<MathFoldContext<'_>>,
) -> Option<LatticeValue> {
    let registry = folds?.registry;
    let binding = tokens.source_binding.as_ref()?;
    let invocation = crate::registry_invocation::normal_transfer_invocation(
        registry,
        registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile),
        tokens,
    )?;
    let (spelling, word) = invocation.stored_value_operand(&binding.variable_context, registry)?;
    if let Some(value) = fold_retained_expression_word(
        word,
        tokens,
        statement,
        values,
        source,
        RetainedExpressionFoldInputs {
            registry,
            policy,
            math,
        },
    ) {
        return Some(value);
    }
    if let Some(value) = invocation.stored_value_literal(&binding.variable_context, registry) {
        return Some(LatticeValue::Const(parse_literal_value(&value)));
    }
    if source.is_positioned() && word.sole_variable_substitution().is_some() {
        return Some(
            source
                .read_word(word)
                .and_then(|read| read.version.map(|version| (read.symbol, version)))
                .map_or(LatticeValue::Overdefined, |key| {
                    values.get(&key).cloned().unwrap_or(LatticeValue::Unknown)
                }),
        );
    }
    Some(fold_assign_value(
        spelling,
        &statement.uses,
        values,
        source,
        policy,
        folds,
    ))
}

fn single_iteration_binding(
    command: &str,
    args: &[String],
    defs: &[String],
    tokens: Option<&crate::ir::CommandTokens>,
) -> bool {
    defs.len() == 1
        && args.len() == 1
        && tokens.map_or_else(
            || matches!(command, "foreach" | "lmap"),
            |tokens| {
                matches!(
                    tokens.synthetic,
                    Some(crate::ir::SyntheticMarker::IterationBindings(_))
                )
            },
        )
}

/// Resolve `$var` / `${var}` to a lattice value by looking up the
/// SSA version in `uses` and indexing `values`. Returns None when
/// the text isn't a simple var reference.
fn source_read_key<S: std::hash::BuildHasher>(
    name: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S>,
    source: SsaSourceView<'_>,
) -> Option<ValueKey> {
    if source.is_positioned() {
        let read = source.read_spelling(name)?;
        Some((read.symbol, read.version?))
    } else {
        let symbol = source.symbol(name)?;
        Some((symbol, *uses.get(&symbol)?))
    }
}

fn resolve_simple_var_ref<'a, S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    text: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: impl Into<SsaSourceView<'a>>,
) -> Option<LatticeValue> {
    let ssa = ssa.into();
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
    let key = source_read_key(name, uses, ssa)?;
    Some(values.get(&key).cloned().unwrap_or(LatticeValue::Unknown))
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
struct BranchFold<'a> {
    math_dependencies: Option<&'a MathDependencies>,
    registry: &'a CommandRegistry,
    policy: FoldPolicy,
    grammar: tcl_dialect::LexerGrammar,
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
    loop_summary_decision(cfg, ssa, bn, condition, values, fold).or_else(|| {
        let math = branch_math_bindings(cfg, bn);
        let math = if fold
            .math_dependencies
            .is_some_and(|ledger| ledger.purpose == SolverPurpose::SemanticAnalysis)
        {
            materialised_branch_bindings(cfg, bn, math)
        } else {
            math
        };
        evaluate_branch_with_math(
            ssa_block,
            condition,
            values,
            BranchExpressionPolicy {
                policy: fold.policy,
                grammar: fold.grammar,
            },
            ssa,
            Some(MathFoldContext {
                point: Some(ExpressionEvaluationPoint::Branch { block: bn }),
                bindings: math,
                dependencies: fold.math_dependencies,
                incoming_reads: Some((SsaSourceView::at_terminator(ssa, bn), fold.registry)),
            }),
        )
    })
}

fn branch_math_bindings(cfg: &CfgFunction, block: BlockId) -> ExpressionMathBindings<'_> {
    let base = match cfg
        .blocks
        .get(&block)
        .and_then(|block| block.terminator.as_ref())
    {
        Some(Terminator::Branch { condition_base, .. }) => *condition_base,
        _ => None,
    };
    ExpressionMathBindings::for_origin(
        &cfg.implicit_math_invocations,
        cfg.terminator_sources
            .get(&block)
            .and_then(Option::as_deref),
        base,
    )
    .with_preparations(&cfg.expression_preparations)
}

/// A dynamic condition word is prepared from its actual evaluated bytes.
/// Only analysis may use that source object: its fresh tree does not prove
/// that discarding the original argv/read/conversion effects is safe.
fn materialised_branch_bindings<'a>(
    cfg: &'a CfgFunction,
    block: BlockId,
    bindings: ExpressionMathBindings<'a>,
) -> ExpressionMathBindings<'a> {
    if bindings.preparation().is_some() {
        return bindings;
    }
    cfg.source_input_tokens_at(block, usize::MAX)
        .and_then(|tokens| tokens.argv.first())
        .and_then(|span| bindings.at_invocation(span.start()))
        .unwrap_or(bindings)
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
    fold: BranchFold<'_>,
) -> Option<bool> {
    let policy = fold.policy;
    let node = cfg.loop_nodes.get(&bn)?;
    let start_ssa = ssa.blocks.get(&node.entry_block)?;
    let mut start_env = crate::static_loops::StaticEnv::new();
    let lookup = if ssa.point_contexts.is_some() {
        SsaSourceView::at_terminator(ssa, node.entry_block)
    } else {
        SsaSourceView::unpositioned(ssa)
    };
    if lookup.is_positioned() {
        for (name, reference) in lookup.reaching_bindings(fold.registry) {
            let Some(version) = reference.version else {
                continue;
            };
            if let Some(LatticeValue::Const(value)) = values.get(&(reference.symbol, version)) {
                start_env.insert(name.to_owned(), const_to_static(value));
            }
        }
    } else {
        for (name, symbol) in lookup.source_symbols() {
            let Some(version) = start_ssa.exit_versions.get(&symbol) else {
                continue;
            };
            if let Some(LatticeValue::Const(value)) = values.get(&(symbol, *version)) {
                start_env.insert(name.to_owned(), const_to_static(value));
            }
        }
    }
    let condition_base = match &node.for_stmt {
        crate::ir::Statement::For { condition_base, .. } => *condition_base,
        _ => None,
    };
    let loop_math = ExpressionMathBindings::for_origin(
        &cfg.implicit_math_invocations,
        node.executed_source.as_deref(),
        condition_base,
    )
    .with_preparations(&cfg.expression_preparations);
    if fold
        .math_dependencies
        .is_some_and(|ledger| ledger.purpose == SolverPurpose::SemanticAnalysis)
    {
        return semantic_loop_summary_decision(
            cfg,
            bn,
            condition,
            &node.for_stmt,
            &start_env,
            loop_math,
            fold,
        );
    }
    let summarised = crate::static_loops::summarise_for_statement_with_dependencies(
        &node.for_stmt,
        &start_env,
        crate::static_loops::DEFAULT_MAX_STATIC_LOOP_ITERS,
        policy,
        Some(loop_math),
    )?;
    let v =
        crate::static_loops::evaluate_expr_with_constants(condition, &summarised.values, policy)?;
    if let Some(ledger) = fold.math_dependencies
        && !ledger.retain(
            &summarised.required_math_invocations,
            &summarised.required_expression_preparations,
        )
    {
        return None;
    }
    Some(v != 0)
}

fn semantic_loop_summary_decision(
    cfg: &CfgFunction,
    block: BlockId,
    condition: &ExprNode,
    statement: &Statement,
    initial: &crate::static_loops::StaticEnv,
    bindings: ExpressionMathBindings<'_>,
    fold: BranchFold<'_>,
) -> Option<bool> {
    let ledger = fold.math_dependencies?;
    let point = ExpressionEvaluationPoint::Branch { block };
    ledger.loops.borrow_mut().remove(&point);
    let analysis = crate::static_loops::analyse_for_statement_with_dependencies(
        statement,
        initial,
        crate::static_loops::DEFAULT_MAX_STATIC_LOOP_ITERS,
        fold.policy,
        Some(bindings),
    )?;
    let environment = analysis
        .values
        .iter()
        .map(|(name, value)| (name.clone(), value.to_env_value()))
        .collect();
    let FoldedExpressionContents::Numeric(value) = fold_math_expression(
        condition,
        &environment,
        fold.policy,
        Some(MathFoldContext {
            point: Some(point),
            bindings: branch_math_bindings(cfg, block),
            dependencies: Some(ledger),
            incoming_reads: None,
        }),
    )?
    else {
        return None;
    };
    if matches!(&value, TclValue::Float(value) if value.is_nan())
        || !ledger.retain(
            &analysis.required_math_invocations,
            &analysis.required_expression_preparations,
        )
    {
        return None;
    }
    ledger.loops.borrow_mut().insert(point, analysis);
    Some(value.is_truthy())
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
    grammar: tcl_dialect::LexerGrammar,
) -> Option<bool> {
    evaluate_branch_with_math(
        ssa_block,
        condition,
        values,
        BranchExpressionPolicy { policy, grammar },
        ssa,
        None,
    )
}

#[derive(Clone, Copy)]
struct BranchExpressionPolicy {
    policy: FoldPolicy,
    grammar: tcl_dialect::LexerGrammar,
}

fn evaluate_branch_with_math<S: std::hash::BuildHasher>(
    ssa_block: &crate::ssa::SsaBlock,
    condition: &ExprNode,
    values: &HashMap<ValueKey, LatticeValue, S>,
    fold: BranchExpressionPolicy,
    ssa: &SsaFunction,
    math: Option<MathFoldContext<'_>>,
) -> Option<bool> {
    let BranchExpressionPolicy { policy, grammar } = fold;
    let block = ssa.block_id(&ssa_block.name).unwrap_or(BlockId(u32::MAX));
    let lookup = if ssa.point_contexts.is_some() {
        SsaSourceView::at_terminator(ssa, block)
    } else {
        SsaSourceView::unpositioned(ssa)
    };
    let mut env = env_from_uses(&ssa_block.exit_versions, values, lookup);
    // A parameter read in a branch condition without a local redefinition
    // isn't in `exit_versions` (those carry defined-in-block versions), so
    // its caller-provided version-0 seed never reaches the fold. Bind it
    // here — but only when version 0 is still live (the param is not
    // redefined to another value before the branch).
    for name in crate::var_refs::vars_in_expr(condition, grammar) {
        if env.contains_key(&name) {
            continue;
        }
        let symbol = if lookup.is_positioned() {
            lookup
                .read_spelling(&name)
                .filter(|read| read.version == Some(0))
                .map(|read| read.symbol)
        } else {
            lookup.symbol(&name)
        };
        let Some(sym) = symbol else {
            continue;
        };
        let v0_live = ssa_block.exit_versions.get(&sym).copied().unwrap_or(0) == 0;
        if v0_live && let Some(LatticeValue::Const(c)) = values.get(&(sym, 0)) {
            env.insert(name, const_to_env_value(c));
        }
    }
    let FoldedExpressionContents::Numeric(v) = fold_math_expression(condition, &env, policy, math)?
    else {
        return None;
    };
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
fn env_from_uses<'a, S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: impl Into<SsaSourceView<'a>>,
) -> Env {
    let ssa = ssa.into();
    let mut env = Env::new();
    for (name, _) in ssa.source_symbols() {
        let Some(key) = source_read_key(name, uses, ssa) else {
            continue;
        };
        if let Some(LatticeValue::Const(c)) = values.get(&key) {
            env.insert(name.to_owned(), const_to_env_value(c));
        }
    }
    env
}

/// Like [`env_from_uses`] but includes only variables whose lattice value is
/// *numeric* (int / float / bool). Used for folding a quoted / bare
/// `expr "…"`, where Tcl substitutes the variable's value textually before
/// parsing: a non-numeric value becomes an invalid bareword, so leaving it
/// unbound makes the fold bail (matching Tcl's runtime error).
fn env_from_uses_numeric<'a, S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: impl Into<SsaSourceView<'a>>,
) -> Env {
    let ssa = ssa.into();
    let mut env = Env::new();
    for (name, _) in ssa.source_symbols() {
        let Some(key) = source_read_key(name, uses, ssa) else {
            continue;
        };
        if let Some(LatticeValue::Const(c)) = values.get(&key)
            && matches!(
                c,
                ConstValue::Int(_) | ConstValue::Float(_) | ConstValue::Bool(_)
            )
        {
            env.insert(name.to_owned(), const_to_env_value(c));
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
pub fn resolve_foreach_list_via_lattice<'a, S1, S2>(
    list_text: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: impl Into<SsaSourceView<'a>>,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> Option<Vec<String>>
where
    S1: std::hash::BuildHasher,
    S2: std::hash::BuildHasher,
{
    let ssa = ssa.into();
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
    let sym = ssa.symbol(name)?;
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
/// 1. **Plain literal** — no `$` / `[` → `Const(parse_literal_value)`.
/// 2. **Simple var reference** `$x` / `${x}` → lattice lookup.
/// 3. **Command substitution** `[cmd args…]` → delegate to
///    [`try_fold_cmd_subst`], then (when `folds` is supplied) to the
///    registry const-fold engine ([`BuiltinFoldInputs`]).
///
/// Anything else widens to `Overdefined`.
fn fold_assign_value<'a, S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    value: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: impl Into<SsaSourceView<'a>>,
    policy: FoldPolicy,
    folds: Option<BuiltinFoldInputs<'_>>,
) -> LatticeValue {
    let ssa = ssa.into();
    let stripped = value.trim();
    // Plain literal.
    if !stripped.contains('$') && !stripped.contains('[') {
        return LatticeValue::Const(parse_literal_value(stripped));
    }
    // A retained set value word identifies its exact read. The statement's
    // later spelling map cannot represent alias reselection within argv.
    if let Some(tokens) = ssa.source_tokens()
        && let Some(effective) = crate::registry_invocation::effective_command_words(tokens)
        && let Some(word) = effective.words.get(2)
        && word.sole_variable_substitution().is_some()
    {
        return ssa
            .read_word(word)
            .and_then(|read| read.version.map(|version| (read.symbol, version)))
            .map_or(LatticeValue::Overdefined, |key| {
                values.get(&key).cloned().unwrap_or(LatticeValue::Unknown)
            });
    }
    // Legacy tokenless callers retain the unique-binding compatibility view.
    if let Some(resolved) = resolve_simple_var_ref(stripped, uses, values, ssa) {
        return resolved;
    }
    // Command substitution.
    if stripped.starts_with('[') && stripped.ends_with(']') {
        if let Some(lv) = try_fold_cmd_subst(stripped, uses, values, ssa, policy, folds) {
            return lv;
        }
        // Registry const-fold fallback: the fold's `$var`
        // words resolve at this statement's use versions, so a folded
        // value re-enters the lattice and downstream statements see it —
        // the multi-hop chain the hardcoded arms above cannot close.
        // Checked AFTER them so single-hop results stay byte-identical.
        if let Some(f) = folds.filter(|f| f.registry_engine) {
            let trusts = |name: &str| f.mutations.trusts(name);
            let lookup = |name: &str| lattice_const_text(name, uses, values, ssa, policy);
            if let Some(folded) = (crate::const_subst::ConstSubstCtx {
                registry: f.registry,
                resolution_namespace: "::",
                namespace_context: None,
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
fn lattice_const_text<'a, S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    name: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: impl Into<SsaSourceView<'a>>,
    policy: FoldPolicy,
) -> Option<String> {
    let ssa = ssa.into();
    let key = source_read_key(name, uses, ssa)?;
    match values.get(&key)? {
        LatticeValue::Const(ConstValue::String(s)) => Some(s.clone()),
        LatticeValue::Const(ConstValue::Int(i)) => Some(i.to_string()),
        LatticeValue::Const(ConstValue::Bool(b)) => Some(if *b { "1" } else { "0" }.to_owned()),
        LatticeValue::Const(ConstValue::Float(f)) => {
            crate::tcl_expr_eval::format_tcl_value_with_policy(&TclValue::Float(*f), policy)
        }
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
fn resolve_const_string<'a, S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    arg: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: impl Into<SsaSourceView<'a>>,
    policy: FoldPolicy,
) -> Option<String> {
    let ssa = ssa.into();
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
        let key = source_read_key(name, uses, ssa)?;
        return match values.get(&key)? {
            LatticeValue::Const(ConstValue::String(s)) => Some(s.clone()),
            LatticeValue::Const(ConstValue::Int(i)) => Some(i.to_string()),
            LatticeValue::Const(ConstValue::Bool(b)) => Some(if *b { "1" } else { "0" }.to_owned()),
            LatticeValue::Const(ConstValue::Float(f)) => {
                crate::tcl_expr_eval::format_tcl_value_with_policy(&TclValue::Float(*f), policy)
            }
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
fn try_fold_cmd_subst<'a, S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    value: &str,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    ssa: impl Into<SsaSourceView<'a>>,
    policy: FoldPolicy,
    folds: Option<BuiltinFoldInputs<'_>>,
) -> Option<LatticeValue> {
    let ssa = ssa.into();
    // Each arm below *is* a builtin's semantics, so it may only run while
    // that name still denotes the builtin: after `rename list mylist` or a
    // shadowing `proc format …` anywhere in the unit, `[list a 1 a 2]` is a
    // call to something else entirely. This is the same trust fact the
    // registry-driven engine below consults; these arms run ahead of it, so
    // they must ask for themselves.
    //
    // Absent `folds` the caller holds **no** whole-module command view, so
    // there is no evidence the head still denotes its builtin and the arms
    // decline. An earlier revision inverted that — a missing fact trusted
    // everything — on the premise that no rewrite lands from a lattice built
    // without the fact. That premise was false: the optimiser's projection
    // takes the shared per-unit lattice as its baseline and only *adds* to it
    // from the trusted re-run, so `proc llength {l} {return 99}` had the
    // shared lattice answer `3` for `set v [llength {a b c}]` while the
    // trusted O103 fold answered `99` for the same call (#2164).
    let trusted = |name: &str| {
        folds.is_some_and(|f| match f.trust {
            FoldTrust::WholeModule => f.mutations.trusts(name),
            FoldTrust::ObservedBindings => f.mutations.observed_binding_is_the_builtin(name),
        })
    };

    // `[list ...]` — reuse the codegen fold.
    if trusted("list")
        && let Some(folded) = crate::codegen::helpers::fold_list_cmd(value, policy.word_rules)
    {
        return Some(LatticeValue::Const(ConstValue::String(folded)));
    }
    // `[format "..." args…]` with literal args.
    // The document's escape grammar, from the same resolved profile the rest
    // of the policy's axes come from. A caller with no dialect keeps the 9.0
    // default, which is what `FoldPolicy::numbers` documents for its own axis
    // — the fold stays available, it just stops guessing once a dialect is
    // known.
    let escapes = policy
        .dialect
        .map_or_else(tcl_dialect::EscapeSyntax::default, |profile| {
            profile.grammar.escapes
        });
    if trusted("format")
        && let Some(folded) = crate::codegen::helpers::try_format_fold(value, escapes)
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
        // `{…}`/`"…"` wrapping that `extract_foreach_elements` does not
        // strip — peel exactly one level before splitting.
        if let Some(elements) = extract_foreach_elements(strip_one_level(arg), policy.word_rules) {
            let n = i64::try_from(elements.len()).unwrap_or(i64::MAX);
            return Some(LatticeValue::Const(ConstValue::Int(n)));
        }
        if let Some(items) =
            resolve_foreach_list_via_lattice(arg, uses, values, ssa, policy.word_rules)
        {
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
                && let Some(s) = resolve_const_string(raw.trim(), uses, values, ssa, policy)
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
        let expr = crate::expr_parser::parse_expr_for_profile(expr_text, policy.dialect);
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
        // Only when the literal *is* the integer, with nothing around it.
        // `" 5"` renders back as `5`, losing the space the program keeps.
        && stripped == text
    {
        return ConstValue::Int(i);
    }
    // The exact spelling, not the trimmed one. Trimming here corrupted the
    // value for every consumer of the lattice: `set p { again}` reached the
    // lattice as `again`, so `append s $p` was rewritten to `append s again`
    // and the program printed `helloagain` where tclsh prints `hello again`
    // (#2052). Whitespace inside a Tcl word is data.
    ConstValue::String(text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_math_dependencies_survive_only_successful_reached_evaluations() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        for (source, expected) in [
            ("set x [expr {abs(-3)}]", 1),
            ("set x [expr {0 && abs(-3)}]", 0),
            ("set x [expr {sqrt(-1)}]", 0),
            (
                "proc ::tcl::mathfunc::abs {x} {return 99}; set x [expr {abs(-3)}]",
                0,
            ),
        ] {
            let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
                source, &registry, false, profile,
            );
            let semantic = semantic_facts_for_unit(&unit, &registry);
            let requirements = &semantic.result.required_math_invocations;
            assert_eq!(requirements.len(), expected, "{source}");
            for requirement in requirements {
                assert_eq!(requirement.function, "abs");
                assert!(semantic.expressions.values().any(|expression| {
                    expression.required_math_invocations.contains(requirement)
                }));
            }
            assert!(unit.top_level.sccp.required_math_invocations.is_empty());
        }
    }

    #[test]
    fn successful_lazy_expression_analysis_retains_preparation_without_reached_calls() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            "set x [expr {0 && future_function(1)}]",
            &registry,
            false,
            profile,
        );
        let semantic = semantic_facts_for_unit(&unit, &registry);
        let result = &semantic.result;
        assert_eq!(result.required_math_invocations.len(), 0);
        assert_eq!(result.required_expression_preparations.len(), 1);
        assert!(
            unit.top_level
                .sccp
                .required_expression_preparations
                .is_empty()
        );
        let preparation = &result.required_expression_preparations[0];
        assert_eq!(preparation.witness.source(), "0 && future_function(1)");
        assert!(semantic.expressions.values().any(|expression| {
            expression
                .required_expression_preparations
                .contains(preparation)
        }));
        let bindings = ExpressionMathBindings::for_origin(
            &unit.top_level.cfg.implicit_math_invocations,
            unit.top_level.cfg.executed_source.as_deref(),
            Some(preparation.source.base()),
        )
        .with_preparations(&unit.top_level.cfg.expression_preparations);
        let wrong_profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let dependencies = MathDependencies {
            purpose: SolverPurpose::SemanticAnalysis,
            ..MathDependencies::default()
        };
        assert!(
            fold_math_expression(
                preparation.witness.tree(),
                &Env::new(),
                FoldPolicy::for_profile(Some(true), Some(wrong_profile)),
                Some(MathFoldContext {
                    point: None,
                    bindings,
                    dependencies: Some(&dependencies),
                    incoming_reads: None,
                }),
            )
            .is_none()
        );
        assert_eq!(dependencies.preparations.borrow().len(), 0);
    }

    #[test]
    fn analysis_value_keeps_native_conversion_obligations_out_of_execution_constants() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            "set x 3; expr {$x + 1}",
            &registry,
            false,
            profile,
        );
        let function = &unit.top_level;
        let point = function
            .ssa
            .blocks
            .iter()
            .find_map(|(&block, body)| {
                body.statements
                    .iter()
                    .enumerate()
                    .find_map(|(index, statement)| {
                        matches!(statement.statement, Statement::ExprEval { .. })
                            .then_some(ExpressionEvaluationPoint::Statement { block, index })
                    })
            })
            .expect("retained full expression producer");
        let original_values = function.sccp.values.clone();
        let analysis = function
            .sccp
            .expression_analysis(
                &function.cfg,
                &function.ssa,
                point,
                FoldPolicy::for_profile(Some(false), Some(profile)),
            )
            .expect("known numerical input at exact native read");
        assert_eq!(analysis.evaluation.value, TclValue::Int(4));
        assert_ne!(
            analysis.evaluation.coercions,
            [] as [crate::tcl_expr_eval::NativeCoercionObligation; 0]
        );
        assert!(!analysis.evaluation.native_value_effects_are_proved());
        assert_eq!(analysis.required_expression_preparations.len(), 1);
        assert_eq!(analysis.required_math_invocations.len(), 0);
        assert_eq!(function.sccp.values, original_values);
    }

    fn semantic_facts_for_unit(
        unit: &crate::compilation_unit::CompilationUnit,
        registry: &CommandRegistry,
    ) -> SemanticValueFacts {
        semantic_value_facts(
            &unit.top_level.cfg,
            &unit.top_level.ssa,
            ValueFactInputs {
                param_constants: None,
                policy: FoldPolicy::from_registry(registry),
                extra_escaping: &HashSet::new(),
                trace: TraceInputs {
                    registry,
                    traced_variables: &unit.ir_module.traced_variables,
                    has_dynamic_variable_trace: unit.ir_module.has_dynamic_variable_trace,
                },
                folds: Some(BuiltinFoldInputs {
                    registry,
                    mutations: &unit.command_mutations,
                    dialect: registry.profile(),
                    defining_class: None,
                    registry_engine: false,
                    trust: FoldTrust::ObservedBindings,
                    proven_pure_parameters: false,
                }),
            },
        )
    }

    #[test]
    fn semantic_solver_propagates_numeric_contents_without_erasing_producer_effects() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            "set x 3; set y [expr {$x + 1}]; set z [expr {$y + 1}]; expr {$z + 1}",
            &registry,
            false,
            profile,
        );
        let facts = semantic_facts_for_unit(&unit, &registry);
        let analyses: Vec<_> = facts.expressions.values().collect();
        for expected in [4, 5, 6] {
            assert!(
                analyses
                    .iter()
                    .any(|analysis| analysis.evaluation.value == TclValue::Int(expected)),
                "missing chained value {expected}: {facts:#?}"
            );
        }
        assert!(
            analyses
                .iter()
                .all(|analysis| !analysis.evaluation.coercions.is_empty())
        );
        assert!(
            analyses
                .iter()
                .all(|analysis| analysis.required_expression_preparations.len() == 1)
        );
        let final_point = unit
            .top_level
            .ssa
            .blocks
            .iter()
            .find_map(|(&block, body)| {
                body.statements
                    .iter()
                    .enumerate()
                    .find_map(|(index, statement)| {
                        matches!(statement.statement, Statement::ExprEval { .. })
                            .then_some(ExpressionEvaluationPoint::Statement { block, index })
                    })
            })
            .expect("retained standalone expression");
        assert_eq!(
            facts.expression(final_point).unwrap().evaluation.value,
            TclValue::Int(6)
        );
        assert!(
            unit.top_level
                .sccp
                .expression_analysis(
                    &unit.top_level.cfg,
                    &unit.top_level.ssa,
                    final_point,
                    FoldPolicy::from_registry(&registry),
                )
                .is_none()
        );
    }

    #[test]
    fn native_result_protocols_do_not_become_contents_only_execution_constants() {
        for selected in ["tcl8.4", "tcl8.6", "jim"] {
            let owner = tcl_registry::model::ingress::static_context_for(selected);
            let registry = owner.commands();
            let profile = registry.profile().expect("actual selected native profile");
            let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
                "set result [expr {2 + 2}]",
                registry,
                false,
                profile,
            );
            let semantic = semantic_facts_for_unit(&unit, registry);
            let producer = unit
                .top_level
                .ssa
                .blocks
                .values()
                .flat_map(|block| &block.statements)
                .find(|statement| {
                    matches!(&statement.statement, Statement::AssignExpr { name, .. } if name == "result")
                })
                .expect("retained original arithmetic result producer");
            assert!(!producer.defs.is_empty());
            for (&symbol, &version) in &producer.defs {
                let key = (symbol, version);
                assert_eq!(
                    semantic.contents(key),
                    Some(&LatticeValue::Const(ConstValue::Int(4))),
                    "numeric contents remain available for {selected}",
                );
                assert!(
                    !matches!(
                        unit.top_level.sccp.values.get(&key),
                        Some(LatticeValue::Const(_))
                    ),
                    "native arithmetic/pool result cannot become a raw literal for {selected}",
                );
            }
        }
    }

    #[test]
    fn semantic_solver_keeps_selected_native_result_bytes_separate_from_number() {
        let profile = tcl_registry::model::ingress::static_context_for("jim")
            .commands()
            .profile()
            .unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            "set x 003; set y [expr {$x}]; expr {$y + 1}",
            &registry,
            false,
            profile,
        );
        let facts = semantic_facts_for_unit(&unit, &registry);
        let selected = facts.expressions.values().find(|analysis| {
            matches!(analysis.evaluation.result_dependency,
                Some(crate::tcl_expr_eval::NativeExpressionResultDependency::SelectedOperand { .. }))
        }).expect("retained selected-object producer");
        assert_eq!(selected.evaluation.value, TclValue::Int(3));
        assert!(matches!(&selected.evaluation.result_dependency,
            Some(crate::tcl_expr_eval::NativeExpressionResultDependency::SelectedOperand {
                existing_bytes: Some(bytes), ..
            }) if bytes == "003"));
        assert!(
            facts.result.values.values().any(|value| {
                *value == LatticeValue::Const(ConstValue::String("003".to_owned()))
            })
        );
        assert!(
            facts
                .expressions
                .values()
                .any(|analysis| analysis.evaluation.value == TclValue::Int(4))
        );
    }

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
            expr_base: None,
            tokens: None,
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
            expr_base: None,
            tokens: None,
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

    #[test]
    fn point_expression_environment_preserves_alias_spelling_and_retargeting() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let source =
            include_str!("../../tcl-syntax/tests/data/resolution/alias-retarget-expression.tcl");
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, registry, false, "tcl9.0",
        );
        let ssa = &unit.procedures["::p"].ssa;
        let mut seen = 0;
        for (&block, body) in &ssa.blocks {
            for (index, statement) in body.statements.iter().enumerate() {
                let (Statement::AssignExpr { name, .. } | Statement::AssignValue { name, .. }) =
                    &statement.statement
                else {
                    continue;
                };
                let expected = match name.as_str() {
                    "first" => 1,
                    "second" => 9,
                    _ => continue,
                };
                let lookup = SsaSourceView::at_statement(ssa, block, index);
                let symbol = lookup.symbol("y").expect("point-specific alias read");
                let version = statement.uses[&symbol];
                let values = HashMap::from([(
                    (symbol, version),
                    LatticeValue::Const(ConstValue::Int(expected)),
                )]);
                let env = env_from_uses(&statement.uses, &values, lookup);
                assert!(
                    matches!(env.get("y"), Some(EnvValue::Int(value)) if *value == expected),
                    "source alias must remain available: {env:?}"
                );
                assert_eq!(
                    env_from_uses(
                        &statement.uses,
                        &values,
                        SsaSourceView::at_statement(ssa, BlockId(u32::MAX), index)
                    )
                    .len(),
                    0,
                    "missing exact point cannot recover a display-name environment"
                );
                seen += 1;
            }
        }
        assert_eq!(
            seen, 2,
            "both source expressions must be exercised: {ssa:#?}"
        );
    }

    /// A literal body resolves its actual selected-frame mutation; a dynamic
    /// body retains unknown contents and presence on its reached continuation.
    #[test]
    fn existence_fold_resolves_literal_upframe_and_declines_dynamic_body() {
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
            f.sccp.constant_branches.iter().any(|branch| branch.value),
            "the literal selected-frame store creates the queried cell: {:?}",
            f.sccp.constant_branches,
        );
        let dynamic = crate::compilation_unit::CompilationUnit::build_for(
            "proc f {script} {uplevel 0 $script; if {[info exists created]} {return yes} else {return no}}",
            &registry,
            false,
        );
        assert!(
            dynamic
                .function("::f")
                .unwrap()
                .sccp
                .constant_branches
                .is_empty(),
            "a caller-supplied body may create or remove the queried cell",
        );
    }

    #[test]
    fn existence_fold_resolves_nested_upframe_alias_unset_mutation() {
        // `uplevel 0` evaluates in this procedure's frame. The nested body
        // aliases that frame's parameter and unsets it, so Tcl observes the else
        // branch. The no-uplevel twin proves the branch is otherwise foldable;
        // this tests the resolved mutation rather than a whole-function barrier.
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
        let branches = &f.sccp.constant_branches;
        assert!(
            branches.iter().any(|branch| !branch.value),
            "the literal alias unset removes the selected local: {branches:?}",
        );
        assert!(branches.iter().all(|branch| !branch.value));
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
        assert!(sccp_process_statements(
            &mut values,
            &block,
            &ssa,
            StatementInputs {
                clobbers: None,
                store: &StoreInputs {
                    math_dependencies: None,
                    cfg: None,
                    escaping: &escaping,
                    policy: FoldPolicy::default(),
                    has_dynamic_variable_trace: false,
                    folds: None,
                    registry: &CommandRegistry::build_default(),
                },
            }
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
            expr_base: None,
            tokens: None,
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
            expr_base: None,
            tokens: None,
            value: None,
            value_word: None,
            span: None,
            expr: None,
            braced: false,
        });
        f.blocks.get_mut(&e).unwrap().terminator = Some(Terminator::Return {
            expr_base: None,
            tokens: None,
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
            expr_base: None,
            tokens: None,
            value: None,
            value_word: None,
            span: None,
            expr: None,
            braced: false,
        });
        f.blocks.get_mut(&e).unwrap().terminator = Some(Terminator::Return {
            expr_base: None,
            tokens: None,
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
            expr_base: None,
            tokens: None,
            value: None,
            value_word: None,
            span: None,
            expr: None,
            braced: false,
        });
        f.blocks.get_mut(&e).unwrap().terminator = Some(Terminator::Return {
            expr_base: None,
            tokens: None,
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
        assert_eq!(r.constant_branches.len(), 0);
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
    fn evaluate_def_assign_expr_retains_numeric_precision_and_conversion() {
        let unit = cu("set a 2; set x [expr {$a + 3}]");
        let facts = unit.top_level.semantic_values().unwrap();
        let analysis = facts
            .expression_iter()
            .find_map(|(_, analysis)| {
                (analysis.evaluation.value == TclValue::Int(5)).then_some(analysis)
            })
            .expect("the actual prepared expression computes 5");
        assert_ne!(
            analysis.evaluation.coercions,
            [] as [crate::tcl_expr_eval::NativeCoercionObligation; 0]
        );
        assert!(
            !unit
                .top_level
                .sccp
                .values
                .values()
                .any(|value| { *value == LatticeValue::Const(ConstValue::Int(5)) }),
            "a contents value alone cannot erase its retained-object conversion"
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
                foreach_groups: None,
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
            evaluate_def(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
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

        let computed = mutations_for("rename $a {}\n");
        assert!(
            computed.has_dynamic_mutation(),
            "an unresolved rename subject raises the unbounded top"
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

        // A value actually established by the shared source owner resolves
        // the same operand to an exact mutation, without widening unrelated names.
        let exact = mutations_for("set a llength\nrename $a {}\n");
        assert!(
            !exact.has_dynamic_mutation(),
            "resolved subject is an exact rename"
        );
        assert!(!exact.observed_binding_is_the_builtin("llength"));
        assert_eq!(
            evaluate_under_lattice_stance(&stmt, &ssa, &reg, &exact),
            LatticeValue::Overdefined
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
        // U+1D11E is one Tcl 9 scalar but two Tcl 8 `Tcl_UniChar` units, so the
        // compile-time fold must answer as the selected runtime would — and
        // decline when no release is selected, leaving the width ambiguous.
        let mut ssa = bare_ssa();
        let stmt = assign_value_stmt(&mut ssa, "n", "[string length \"\u{1D11E}\"]", 1);
        let fold = |dialect: Option<&'static tcl_dialect::DialectProfile>| {
            evaluate_pristine(
                &stmt,
                &HashMap::new(),
                &ssa,
                FoldPolicy::for_profile(Some(false), dialect),
            )
        };
        assert_eq!(
            fold(Some(
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()
            )),
            LatticeValue::Const(ConstValue::Int(1))
        );
        assert_eq!(
            fold(Some(
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
            )),
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
    fn quoted_expr_with_numeric_var_retains_known_contents() {
        let unit = cu(r#"set a 3; set b 4; set r [expr "$a + $b"]"#);
        let facts = unit.top_level.semantic_values().unwrap();
        let known_seven = facts
            .contents_iter()
            .any(|(_, value)| *value == LatticeValue::Const(ConstValue::Int(7)));
        if !known_seven {
            eprintln!(
                "quoted expr statement kinds={:?} failure={} admission={}",
                unit.ir_module
                    .top_level
                    .statements
                    .iter()
                    .map(std::mem::discriminant)
                    .collect::<Vec<_>>(),
                unit.ir_module
                    .top_level
                    .native_compilation_failure
                    .is_some(),
                unit.ir_module
                    .top_level
                    .native_compilation_admission
                    .is_some()
            );
            let show = |binding: &crate::command_binding::SourceInvocationBinding| {
                format!(
                    "targets={:?} unknown={} absent={} residual={:?} handler={:?} bindings={} traces={} argv={:?} compiled={:?}",
                    binding
                        .targets
                        .iter()
                        .map(|target| (&target.command, target.registry_backed))
                        .collect::<Vec<_>>(),
                    binding.unknown,
                    binding.may_be_absent,
                    binding.compiled_execution_residual,
                    binding
                        .native_handler_envelope
                        .as_ref()
                        .map(|target| &target.command),
                    binding.variable_context.dynamic_bindings,
                    binding.variable_context.dynamic_traces,
                    binding.evaluated_argument_values,
                    binding
                        .compiled_candidates
                        .iter()
                        .map(|proof| (proof.operation, proof.certainty))
                        .collect::<Vec<_>>()
                )
            };
            for site in unit.ir_module.top_level.command_binding_sites.iter() {
                if let Some(tokens) = &site.source_tokens {
                    eprintln!(
                        "quoted expr point {:?} argv={:?} proof={:?}",
                        site.span,
                        tokens.argv_texts,
                        tokens.source_binding.as_ref().map(show)
                    );
                    for (offset, binding) in &tokens.nested_bindings {
                        eprintln!("quoted expr nested {offset} {}", show(binding));
                    }
                }
            }
            for proof in &unit.top_level.cfg.expression_preparations {
                eprintln!(
                    "quoted expr preparation site={} base={} bytes={:?} axes={:?}",
                    proof.invocation.offset,
                    proof.source.base(),
                    proof.witness.source(),
                    proof.witness.context()
                );
            }
        }
        assert!(
            known_seven,
            "textual substitution computes native contents 7: {facts:#?}"
        );
        assert!(
            !unit
                .top_level
                .sccp
                .values
                .values()
                .any(|value| { *value == LatticeValue::Const(ConstValue::Int(7)) }),
            "unproved preparation/producer effects do not license erasure"
        );
    }

    #[test]
    fn materialised_condition_values_do_not_license_branch_erasure() {
        let unit = cu("set n 1; if $n {set result YES} else {set result NO}");
        let function = &unit.top_level;
        let known = function.semantic_values().unwrap();
        assert!(
            known
                .result
                .constant_branches
                .iter()
                .any(|branch| { branch.condition == "$n" && branch.value })
        );
        assert_eq!(function.sccp.constant_branches, [] as [ConstantBranch; 0]);
        let analysis = known
            .expression_iter()
            .find_map(|(point, analysis)| {
                matches!(point, ExpressionEvaluationPoint::Branch { .. }).then_some(analysis)
            })
            .expect("the reached condition retains its actual preparation");
        assert_eq!(analysis.required_expression_preparations.len(), 1);
        assert_eq!(
            analysis.required_expression_preparations[0]
                .witness
                .source(),
            "1"
        );
    }

    #[test]
    fn braced_expr_with_string_var_retains_numeric_comparison_obligations() {
        let unit = cu("set a alpha; set b beta; set r [expr {$a == $b}]");
        let facts = unit.top_level.semantic_values().unwrap();
        let analysis = facts
            .expression_iter()
            .find_map(|(_, analysis)| {
                (analysis.evaluation.value == TclValue::Int(0)).then_some(analysis)
            })
            .expect("the actual prepared string comparison computes false");
        assert_ne!(
            analysis.evaluation.coercions,
            [] as [crate::tcl_expr_eval::NativeCoercionObligation; 0]
        );
        assert!(
            !unit
                .top_level
                .sccp
                .values
                .values()
                .any(|value| { *value == LatticeValue::Const(ConstValue::Int(0)) }),
            "numeric comparison may change shared string representations"
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
            evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
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
            expr_base: None,
            tokens: None,
            value: None,
            value_word: None,
            span: None,
            expr: None,
            braced: false,
        });
        f.blocks.get_mut(&dead).unwrap().terminator = Some(Terminator::Return {
            expr_base: None,
            tokens: None,
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
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        crate::compilation_unit::CompilationUnit::build_for_profile(src, &registry, false, profile)
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
        let r = &fu.semantic_values().unwrap().result;
        let cb = r
            .constant_branches
            .iter()
            .find(|cb| cb.condition.contains("$i == 10"))
            .unwrap_or_else(|| {
                panic!(
                    "post-loop branch must fold via the static-loop summary: facts={:#?} loops={:#?} preparations={:#?} sources={:#?}",
                    fu.semantic_values(),
                    fu.cfg.loop_nodes,
                    fu.cfg.expression_preparations,
                    fu.cfg.terminator_sources
                )
            });
        assert!(cb.value, "i == 10 after the loop, so the branch is true");
        let facts = fu.semantic_values().unwrap();
        assert!(
            facts.loops.values().any(|analysis| {
                !analysis.expression_obligations.is_empty()
                    && !analysis.increment_obligations.is_empty()
            }),
            "semantic loop contents retain native operations in evaluation order"
        );
        assert!(
            !fu.sccp
                .constant_branches
                .iter()
                .any(|branch| branch.condition.contains("$i == 10")),
            "numeric simulation does not erase loop or branch conversions"
        );

        // Body side effects are simulated too: j accumulates to 5.
        let ca = cu(
            "proc ::a {} { set j 0\n for {set k 5} {$k > 0} {incr k -1} { incr j }\n if {$j == 5} { return yes } else { return no } }",
        );
        let fa = ca.function("::a").unwrap();
        let ra = &fa.semantic_values().unwrap().result;
        let cba = ra
            .constant_branches
            .iter()
            .find(|cb| cb.condition.contains("$j == 5"))
            .expect("accumulator branch must fold via the static-loop summary");
        assert!(cba.value, "j == 5 after the loop");
        assert!(
            !fa.sccp
                .constant_branches
                .iter()
                .any(|branch| branch.condition.contains("$j == 5")),
            "accumulator precision is not execution-erasure proof"
        );

        // A loop with an unknown (parameter) bound cannot be summarised, so the
        // post-loop branch stays unfolded (conservative).
        let cq = cu(
            "proc ::q {n} { for {set i 0} {$i < $n} {incr i} {}\n if {$i == 10} { return yes } else { return no } }",
        );
        let fq = cq.function("::q").unwrap();
        let rq = &fq.semantic_values().unwrap().result;
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
        let r = sccp_no_traces(&fu.cfg, &fu.ssa, None, FoldPolicy::default());
        let const_sets: Vec<_> = r
            .values
            .iter()
            .filter(|((symbol, _), _)| fu.ssa.var_name(*symbol) == "v")
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
    fn canonical_root_cells_fold_until_a_proved_mutation_dependency() {
        let stable = cu("set a 3; set b 4; if {$a+$b == 7} {set result YES} else {set result NO}");
        assert!(
            !stable
                .top_level
                .semantic_values()
                .unwrap()
                .result
                .constant_branches
                .is_empty(),
            "root-qualified identity alone does not imply mutation; values={:?}; cells={:?}",
            stable.top_level.sccp.values,
            stable.top_level.ssa.cell_names()
        );
        assert!(
            stable.top_level.sccp.constant_branches.is_empty(),
            "known branch values do not prove retained coercion erasure"
        );
        let observed = cu(
            "set a 3; proc observer {args} {set ::a 4}; trace add variable a read observer; if {$a == 3} {set result YES} else {set result NO}",
        );
        assert!(
            observed
                .top_level
                .semantic_values()
                .unwrap()
                .result
                .constant_branches
                .is_empty(),
            "a read observer may mutate the stored value"
        );
        let mutated = cu(
            "set a 3; proc mutate {} {global a; set a 4}; mutate; if {$a == 3} {set result YES} else {set result NO}",
        );
        assert!(
            !mutated
                .top_level
                .semantic_values()
                .unwrap()
                .result
                .constant_branches
                .iter()
                .any(|branch| branch.value),
            "re-entry must never reuse the stale root value"
        );
    }

    #[test]
    fn positioned_constants_do_not_cross_unrepresented_mutations_or_observers() {
        for source in [
            "proc f {cmd} {set x 3; $cmd; if {$x == 3} {return YES} else {return NO}}",
            "proc f {n} {set x 3; set $n 4; if {$x == 3} {return YES} else {return NO}}",
            "proc f {n} {set x 3; unset $n; if {$x == 3} {return YES} else {return NO}}",
            "proc f {n} {set x 3; trace add variable $n read observer; if {$x == 3} {return YES} else {return NO}}",
        ] {
            let unit = cu(source);
            let function = unit.function("::f").expect("procedure retained");
            assert!(
                function
                    .sccp
                    .constant_branches
                    .iter()
                    .all(|branch| !branch.value),
                "an uncertain later read must not reuse the earlier value: {source}; branches={:?}",
                function.sccp.constant_branches,
            );
        }
    }

    #[test]
    fn later_opaque_effects_do_not_destroy_an_earlier_branch_proof() {
        let unit = cu(
            "proc f {cmd} {set x 3; if {$x == 3} {set first YES} else {set first NO}; $cmd; if {$x == 4} {set second YES} else {set second NO}}",
        );
        let function = unit.function("::f").expect("procedure retained");
        assert!(
            function.sccp.constant_branches.is_empty(),
            "unknown retained conversions prevent execution erasure"
        );
        assert!(
            function
                .semantic_values()
                .unwrap()
                .result
                .constant_branches
                .iter()
                .any(|branch| branch.value && branch.condition.contains('3')),
            "the earlier read remains precise: {:?}",
            function.semantic_values().unwrap().result.constant_branches
        );
        assert!(
            !function
                .semantic_values()
                .unwrap()
                .result
                .constant_branches
                .iter()
                .any(|branch| branch.value && branch.condition.contains('4')),
            "the later read has an unknown contents origin: {:?}",
            function.semantic_values().unwrap().result.constant_branches
        );
    }

    #[test]
    fn global_alias_contents_are_precise_until_an_actual_reentry_dependency() {
        // Both controls are independently executed in the six-engine temporal
        // vector corpus. Sharing a cell alone does not change its contents.
        let direct =
            cu("proc p {} {global g; set g 5; if {$g == 5} {return YES} else {return NO}}; p");
        let function = direct.function("::p").unwrap();
        assert!(
            function.sccp.constant_branches.is_empty(),
            "alias contents alone do not erase numeric read conversions"
        );
        assert!(
            function
                .semantic_values()
                .unwrap()
                .result
                .constant_branches
                .iter()
                .any(|branch| branch.value),
            "an unobserved literal store followed immediately by its read is precise"
        );

        let mutated = cu(
            "proc mutate {} {global g; set g 6}; proc p {} {global g; set g 5; mutate; if {$g == 5} {return YES} else {return NO}}; p",
        );
        let function = mutated.function("::p").unwrap();
        assert!(
            !function
                .semantic_values()
                .unwrap()
                .result
                .constant_branches
                .iter()
                .any(|branch| branch.value),
            "the alias read must use the contents after reentry"
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
        assert_eq!(
            r.constant_branches.len(),
            0,
            "a value reachable through an UpFrame must not fold a constant branch, got {:?}",
            r.constant_branches,
        );
    }
}
