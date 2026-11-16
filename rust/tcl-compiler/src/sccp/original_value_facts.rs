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

#[cfg(test)]
use crate::analyses::MAX_CONSTSET_SIZE;
use crate::analyses::{ConstValue, LatticeValue};
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::codegen_helpers::split_list_values;
use crate::expr_ast::ExprNode;
use crate::ir::Statement;
use crate::math_function_binding::ExpressionMathBindings;
use crate::ssa::{SsaFunction, SsaSourceView, SsaStatement, Symbol, ValueKey};
use crate::tcl_expr_eval::{Env, EnvValue, FoldPolicy, TclValue, eval_tcl_expr_with_policy};

use super::{
    BuiltinFoldInputs, ConstantBranch, FoldTrust, SccpResult, TraceInputs, cfg_order,
    compute_predecessors, join, set_value,
};

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
    solve_value_facts(cfg, ssa, &inputs, SolverPurpose::ExecutionErasure).result
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
    let solved = solve_value_facts(cfg, ssa, &inputs, SolverPurpose::SemanticAnalysis);
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

    /// Plans over the same immutable advisory values and reached source blocks.
    /// They do not replace an executable plan or prove a Native result/store.
    pub(crate) fn template_plans(self) -> &'a [super::TemplatePlanRecord] {
        &self.result.template_plans
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
    source_metadata_input: Option<crate::analyser::ResolvedAnalysisInput>,
    fold_source_metadata_input: Option<crate::analyser::ResolvedAnalysisInput>,
    registry: tcl_registry::RegistrySnapshot,
    fold_registry: Option<tcl_registry::RegistrySnapshot>,
    policy: FoldPolicy,
    param_constants: Option<HashMap<(String, crate::ssa::Version), LatticeValue>>,
    extra_escaping: HashSet<String>,
    traced_variables: BTreeSet<String>,
    has_dynamic_variable_trace: bool,
    deferred_writes: crate::ir::DeferredWrites,
    mutations: Option<crate::command_binding::ModuleCommandMutations>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    defining_class: Option<String>,
    registry_engine: bool,
    trust: FoldTrust,
    proven_pure_parameters: bool,
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

    /// Readonly currency of the actual construction inputs, independently of
    /// whether the lazy contents have already been computed. This grants no
    /// executable replacement, handler, frame or result representation.
    #[must_use]
    pub(crate) fn matches_source_input(
        &self,
        registry: &tcl_registry::CommandRegistry,
        source: &crate::analyser::ResolvedAnalysisInput,
    ) -> bool {
        let Some(input) = self.inputs.as_ref() else {
            return false;
        };
        let key = registry.snapshot().semantic_key();
        input.source_metadata_input.as_ref() == Some(source)
            && input.registry.semantic_key() == key
            && crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
                registry, source,
            )
            .is_some()
            && match (&input.fold_registry, &input.fold_source_metadata_input) {
                (None, None) => true,
                (Some(folds), Some(fold_source)) => {
                    folds.semantic_key() == key && fold_source == source
                }
                _ => false,
            }
    }

    /// Retain the exact construction inputs without running another solver.
    #[must_use]
    pub fn new(inputs: ValueFactInputs<'_>) -> Self {
        let folds = inputs.folds;
        Self {
            inputs: Some(OwnedValueFactInputs {
                source_metadata_input: inputs.trace.source_metadata_input.cloned(),
                fold_source_metadata_input: folds
                    .and_then(|folds| folds.source_metadata_input.cloned()),
                registry: inputs.trace.registry.snapshot(),
                fold_registry: folds.map(|folds| folds.registry.snapshot()),
                policy: inputs.policy,
                param_constants: inputs.param_constants.cloned(),
                extra_escaping: inputs.extra_escaping.clone(),
                traced_variables: inputs.trace.traced_variables.clone(),
                has_dynamic_variable_trace: inputs.trace.has_dynamic_variable_trace,
                deferred_writes: inputs.trace.deferred_writes.clone(),
                mutations: folds.map(|folds| folds.mutations.clone()),
                dialect: folds.and_then(|folds| folds.dialect),
                defining_class: folds.and_then(|folds| folds.defining_class.map(str::to_owned)),
                registry_engine: folds.is_some_and(|folds| folds.registry_engine),
                trust: folds.map_or(FoldTrust::ObservedBindings, |folds| folds.trust),
                proven_pure_parameters: folds.is_some_and(|folds| folds.proven_pure_parameters),
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
                        deferred_writes: &input.deferred_writes,
                        analysis_context: None,
                        existence: None,
                        source_metadata_input: input.source_metadata_input.as_ref(),
                        registry,
                        traced_variables: &input.traced_variables,
                        has_dynamic_variable_trace: input.has_dynamic_variable_trace,
                    },
                    folds: input
                        .mutations
                        .as_ref()
                        .zip(input.fold_registry.as_ref())
                        .map(|(mutations, fold_registry)| BuiltinFoldInputs {
                            source_metadata_input: input.fold_source_metadata_input.as_ref(),
                            registry: fold_registry.registry(),
                            mutations,
                            dialect: input.dialect,
                            defining_class: input.defining_class.as_deref(),
                            registry_engine: input.registry_engine,
                            trust: input.trust,
                            proven_pure_parameters: input.proven_pure_parameters,
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
    inputs: &ValueFactInputs<'_>,
    purpose: SolverPurpose,
) -> SolvedValueFacts {
    let ValueFactInputs {
        param_constants,
        policy,
        extra_escaping,
        trace,
        ..
    } = *inputs;
    let preds = compute_predecessors(cfg);
    let (values, grammar) = seed_value_facts(cfg, ssa, param_constants, policy, trace.registry);

    // Retained point contexts describe actual read and write observers and
    // contents origins. A later alias, opaque call or unreachable branch must
    // not retroactively erase an earlier definition. Carrierless compatibility
    // SSA still uses the conservative whole-function exposure summary.
    let mut escaping = compatibility_escaping(cfg, ssa, trace, extra_escaping);
    // A name a variable trace or a stored callback writes changes whenever
    // they run (`after idle {set ::go 0}; update`), which no point context
    // states; the execution solver treats them the same way.
    if ssa.point_contexts.is_some() {
        trace.extend_module_escaping(&mut escaping);
    }
    // A block the solve proves never runs accesses nothing, so the opaque
    // native accesses of such a block (an arm the walk proves dead, whose
    // calls it binds nothing for) leave the running code's values alone.
    // Solve first without them and again with them only when a block that
    // runs has one, as the execution solver does with its hazards (#2375).
    let mut opaque_native_accesses = false;
    loop {
        let solved = solve_value_facts_once(
            (cfg, ssa),
            (inputs, purpose),
            (&preds, values.clone(), grammar),
            &escaping,
            opaque_native_accesses,
        );
        if opaque_native_accesses
            || !ssa
                .has_opaque_native_accesses_in(&|id| solved.result.executable_blocks.contains(&id))
        {
            return solved;
        }
        opaque_native_accesses = true;
    }
}

fn solve_value_facts_once(
    (cfg, ssa): (&CfgFunction, &SsaFunction),
    (inputs, purpose): (&ValueFactInputs<'_>, SolverPurpose),
    (preds, mut values, grammar): (
        &HashMap<BlockId, HashSet<BlockId>>,
        HashMap<ValueKey, LatticeValue>,
        tcl_dialect::LexerGrammar,
    ),
    escaping: &HashSet<String>,
    opaque_native_accesses: bool,
) -> SolvedValueFacts {
    let ValueFactInputs {
        param_constants,
        policy,
        trace,
        folds,
        ..
    } = *inputs;
    let required_math_invocations = MathDependencies::for_parameters(purpose, param_constants);
    let reference_config =
        crate::value_transfer::LatticeDriver::new(trace, folds, policy, escaping)
            .with_cfg_metadata(&cfg.metadata_context)
            .source_reference_lexer_config();
    let store_inputs = StoreInputs {
        reference_config,
        math_dependencies: Some(&required_math_invocations),
        cfg: Some(cfg),
        escaping,
        policy,
        has_dynamic_variable_trace: trace.writes_any_variable() && ssa.point_contexts.is_none(),
        opaque_native_accesses,
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

                let incoming_exec = incoming_executable_predecessors(*bn, preds, &executable_edges);

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
                    reference_config,
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

    finish_value_facts(
        (cfg, ssa),
        (inputs, purpose, grammar),
        (&order, escaping),
        required_math_invocations,
        SettledValueFactState {
            values,
            executable_blocks,
            executable_edges,
        },
    )
}

struct SettledValueFactState {
    values: HashMap<ValueKey, LatticeValue>,
    executable_blocks: HashSet<BlockId>,
    executable_edges: HashSet<(BlockId, BlockId)>,
}

fn finish_value_facts(
    (cfg, ssa): (&CfgFunction, &SsaFunction),
    (inputs, purpose, grammar): (
        &ValueFactInputs<'_>,
        SolverPurpose,
        tcl_dialect::LexerGrammar,
    ),
    (order, escaping): (&[BlockId], &HashSet<String>),
    required_math_invocations: MathDependencies,
    state: SettledValueFactState,
) -> SolvedValueFacts {
    let ValueFactInputs {
        policy,
        trace,
        folds,
        ..
    } = *inputs;
    let SettledValueFactState {
        values,
        executable_blocks,
        executable_edges,
    } = state;
    let constant_branches = collect_constant_branches(
        cfg,
        ssa,
        &values,
        &executable_blocks,
        order,
        BranchFold {
            math_dependencies: Some(&required_math_invocations),
            policy,
            grammar,
            registry: trace.registry,
            reference_config: crate::value_transfer::LatticeDriver::new(
                trace, folds, policy, escaping,
            )
            .with_cfg_metadata(&cfg.metadata_context)
            .source_reference_lexer_config(),
        },
    );

    // Advisory plans read this purpose's settled values and original source
    // metadata. They never borrow execution-SCCP's option values or results.
    let template_plans = if purpose == SolverPurpose::SemanticAnalysis {
        let driver = crate::value_transfer::LatticeDriver::new(trace, folds, policy, escaping)
            .with_cfg_metadata(&cfg.metadata_context);
        let mut plans = Vec::new();
        for &id in order {
            if executable_blocks.contains(&id)
                && let Some(block) = ssa.blocks.get(&id)
            {
                plans.extend(driver.template_plans_in(ssa, (id, block), &values));
            }
        }
        plans.sort_by_key(|record| (record.span.start(), record.span.end()));
        plans
    } else {
        Vec::new()
    };
    let mut solved = required_math_invocations.finish(
        values,
        executable_blocks,
        executable_edges,
        constant_branches,
    );
    solved.result.template_plans = template_plans;
    solved
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
    let metadata = trace.source_metadata_input.and_then(|input| {
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            trace.registry,
            input,
        )
    });
    // No completed alias query exists without actual metadata. A tokenless
    // compatibility graph therefore treats every named value as observable.
    let Some(metadata) = metadata else {
        return ssa.var_names().iter().cloned().collect();
    };
    let mut names = crate::var_observability::analyse_var_observability_with_metadata_context(
        cfg,
        trace.registry,
        Some(metadata),
    )
    .escaping_var_names();
    names.extend(extra.iter().cloned());
    trace.extend_module_escaping(&mut names);
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
/// `true` if any lattice value changed. Used by [`execution_value_facts`].
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
/// Returns `true` if any lattice value changed. Used by [`execution_value_facts`].
/// Store proof inputs shared by every SCCP entry point.
struct StoreInputs<'a> {
    reference_config: Option<tcl_lexer::LexerConfig>,
    math_dependencies: Option<&'a MathDependencies>,
    cfg: Option<&'a CfgFunction>,
    escaping: &'a HashSet<String>,
    policy: FoldPolicy,
    has_dynamic_variable_trace: bool,
    /// Whether the code the solve treats as running has opaque native
    /// accesses ([`SsaFunction::has_opaque_native_accesses_in`]).
    opaque_native_accesses: bool,
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
    crate::place_bridge::def_places_at(statement, block, index, points, registry)
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

fn widen_unrepresented_statement(
    values: &mut HashMap<ValueKey, LatticeValue>,
    stmt_ssa: &SsaStatement,
    positioned: bool,
) -> bool {
    let mut changed = false;
    let keys: Vec<ValueKey> = if positioned {
        // Exact reads after an unrepresented store have no contents
        // version. Earlier SSA versions remain immutable evidence.
        Vec::new()
    } else {
        values.keys().copied().collect()
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
        reference_config,
        math_dependencies,
        cfg,
        escaping,
        policy,
        has_dynamic_variable_trace,
        opaque_native_accesses,
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
            // puts [expr {$n + 1}]` prints `100`. The value before
            // `uplevel` cannot justify folding that later expression.
            changed |=
                widen_unrepresented_statement(values, stmt_ssa, ssa.point_contexts.is_some());
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
            let evaluated = || {
                evaluate_def_with_math(
                    stmt_ssa,
                    &*values,
                    lookup,
                    policy,
                    folds,
                    math,
                    reference_config,
                )
            };
            let val = if ssa.cell_key(var).is_none_or(|cell| observed.contains(cell))
                || lookup.externally_mutable_given(
                    var,
                    escaping,
                    has_dynamic_variable_trace,
                    opaque_native_accesses,
                    registry,
                ) != Some(false)
                || ssa.is_array_root_refresh_version(var, *ver)
                || stmt_ssa.destruction_defs.contains(&var)
            {
                LatticeValue::Overdefined
            } else if math_dependencies
                .is_some_and(|ledger| ledger.purpose == SolverPurpose::SemanticAnalysis)
                && let Some(value) = lookup.conditional_logical_scalar_store_contents(var, registry)
            {
                LatticeValue::Const(parse_literal_value(&value))
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
                        join(&prev, &evaluated())
                    }
                    None => LatticeValue::Overdefined,
                }
            } else {
                evaluated()
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
    reference_config: Option<tcl_lexer::LexerConfig>,
}

/// Process a block's terminator: mark the matching outgoing edges
/// as executable.  Returns `true` when any new edge / block was
/// added.  Used by [`execution_value_facts`].
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
        reference_config,
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
                    reference_config,
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
/// condition evaluated to a constant lattice value.  Used by
/// [`execution_value_facts`].
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
                kind: super::BranchFactKind::Applied,
                block: cfg.block_name(*bn).to_owned(),
                span: *term_span,
                condition: cond_text,
                value: true,
                taken_target: true_name,
                not_taken_target: false_name,
            }),
            Some(false) => constant_branches.push(ConstantBranch {
                kind: super::BranchFactKind::Applied,
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

fn retain_source_call_name_coverage(
    registry: &CommandRegistry,
    tokens: &crate::ir::CommandTokens,
    defined: &mut FxHashSet<String>,
    unset: &mut FxHashSet<String>,
) {
    let Some(names) = crate::registry_invocation::original_source_name_roles(registry, tokens)
    else {
        return;
    };
    let destination = if names
        .facts()
        .traits
        .contains(tcl_registry::Traits::DESTROYS_VARIABLE)
    {
        unset
    } else {
        defined
    };
    for &(argument, role) in names.roles() {
        if role == tcl_registry::ArgRole::VarWrite
            && let Some(name) = names.original_variable_name_operand(argument)
        {
            destination.insert(name.as_str().to_owned());
            if let Some((root, _)) = tcl_syntax::naming::split_element_ref(name.as_str()) {
                destination.insert(root.to_owned());
            }
        }
    }
}

/// Possible writes and destroys protect whole-body existence folds.
/// Original source-only roles retain unknown completion and frame effects.
fn scan_defined_and_unset(
    cfg: &CfgFunction,
    metadata: ExistenceMetadataInputs<'_>,
) -> (
    FxHashSet<String>,
    FxHashSet<String>,
    crate::dynamic_names::DynamicNameBarrier,
) {
    use crate::dynamic_names::DynamicNameBarrier;
    let mut defined = FxHashSet::default();
    let mut unset = FxHashSet::default();
    let mut unknown = DynamicNameBarrier::default();
    let Some(context) = metadata
        .context
        .filter(|context| context.matches_registry(metadata.registry))
    else {
        return (defined, unset, DynamicNameBarrier::OPAQUE_SCRIPT);
    };
    if cfg.metadata_context.source_analysis_input() != context.source_analysis_input()
        && (!cfg.metadata_context.is_standalone() || context.source_analysis_input().is_some())
    {
        return (defined, unset, DynamicNameBarrier::OPAQUE_SCRIPT);
    }
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            match stmt {
                Statement::AssignConst { .. }
                | Statement::AssignExpr { .. }
                | Statement::AssignValue { .. }
                | Statement::Incr { .. } => {
                    let projection = crate::ir_helpers::structural_variable_write_projection(stmt);
                    unknown.writes |= projection.opaque_variable_frame;
                    for name in projection.literal_names {
                        unknown.writes |= name.contains('\0');
                        defined.insert(
                            tcl_syntax::naming::split_element_ref(&name)
                                .map_or(name.as_str(), |(root, _)| root)
                                .to_owned(),
                        );
                    }
                }
                Statement::Call {
                    command,
                    args,
                    defs,
                    ..
                } => {
                    retain_call_name_writes(
                        (command, args, defs),
                        stmt,
                        cfg,
                        metadata,
                        context,
                        (&mut defined, &mut unset, &mut unknown),
                    );
                }
                _ => {}
            }
        }
    }
    (defined, unset, unknown)
}

fn retain_nominal_call_name_writes(
    command: &str,
    args: &[String],
    metadata: ExistenceMetadataInputs<'_>,
    context: crate::registry_invocation::InvocationMetadataContext<'_>,
    (defined, unset, unknown): (
        &mut FxHashSet<String>,
        &mut FxHashSet<String>,
        &mut crate::dynamic_names::DynamicNameBarrier,
    ),
) {
    use crate::dynamic_names::DynamicNameBarrier;
    use tcl_registry::{ArgRole, Traits};
    // Explicit nominal argv has no original invocation or Native permission.
    let arguments: Vec<_> = args.iter().map(String::as_str).collect();
    let Some(schema) = metadata.registry.resolve_invocation(
        command,
        &arguments,
        Some(context.context().authoring_query()),
    ) else {
        *unknown = (*unknown).union(DynamicNameBarrier::OPAQUE_SCRIPT);
        return;
    };
    let facts = schema.facts();
    if !facts.arg_roles_complete {
        *unknown = (*unknown).union(DynamicNameBarrier::OPAQUE_SCRIPT);
    }
    let destroys = facts.traits.contains(Traits::DESTROYS_VARIABLE);
    let argument_offset = facts.argument_offset;
    for (ordinal, role) in facts.arg_roles {
        if role != ArgRole::VarWrite {
            continue;
        }
        let Some(name) = args.get(argument_offset + usize::from(ordinal)) else {
            *unknown = (*unknown).union(DynamicNameBarrier::OPAQUE_SCRIPT);
            continue;
        };
        if crate::dynamic_names::names_a_dynamic_variable(name) || name.contains('\0') {
            if destroys {
                unknown.destroys = true;
            } else {
                unknown.writes = true;
            }
        } else {
            let destination = if destroys { &mut *unset } else { &mut *defined };
            destination.insert(name.clone());
            if let Some((root, _)) = tcl_syntax::naming::split_element_ref(name) {
                destination.insert(root.to_owned());
            }
        }
    }
}

fn retain_call_name_writes(
    (command, args, defs): (&str, &[String], &[String]),
    stmt: &Statement,
    cfg: &CfgFunction,
    metadata: ExistenceMetadataInputs<'_>,
    context: crate::registry_invocation::InvocationMetadataContext<'_>,
    (defined, unset, unknown): (
        &mut FxHashSet<String>,
        &mut FxHashSet<String>,
        &mut crate::dynamic_names::DynamicNameBarrier,
    ),
) {
    use crate::dynamic_names::DynamicNameBarrier;
    use crate::registry_invocation::EffectiveInvocationWord;
    use tcl_registry::ArgRole;
    defined.extend(defs.iter().cloned());
    if let Some(tokens) = stmt.tokens() {
        let Some(names) =
            crate::registry_invocation::possible_variable_name_operands_with_metadata_context(
                metadata.registry,
                Some(context),
                tokens,
            )
        else {
            *unknown = (*unknown).union(DynamicNameBarrier::OPAQUE_SCRIPT);
            retain_source_call_name_coverage(metadata.registry, tokens, defined, unset);
            return;
        };
        if names.unknown_residual() {
            *unknown = (*unknown).union(DynamicNameBarrier::OPAQUE_SCRIPT);
            retain_source_call_name_coverage(metadata.registry, tokens, &mut *defined, &mut *unset);
        }
        for (role, word, _, destroys) in names.phased_operands() {
            if role != ArgRole::VarWrite {
                continue;
            }
            let destination = if destroys { &mut *unset } else { &mut *defined };
            match word {
                EffectiveInvocationWord::Literal(name) => {
                    destination.insert(name.clone());
                    if let Some((root, _)) = tcl_syntax::naming::split_element_ref(name) {
                        destination.insert(root.to_owned());
                    }
                    if name.contains('\0') {
                        if destroys {
                            unknown.destroys = true;
                        } else {
                            unknown.writes = true;
                        }
                    }
                }
                EffectiveInvocationWord::ArrayElementName { root } => {
                    destination.insert(root.clone());
                }
                _ => {
                    if destroys {
                        unknown.destroys = true;
                    } else {
                        unknown.writes = true;
                    }
                }
            }
        }
        for (role, destroys) in names.phased_unresolved_roles() {
            if role == ArgRole::VarWrite {
                if destroys {
                    unknown.destroys = true;
                } else {
                    unknown.writes = true;
                }
            }
        }
    } else if cfg.metadata_context.is_standalone() && context.source_analysis_input().is_none() {
        retain_nominal_call_name_writes(
            command,
            args,
            metadata,
            context,
            (defined, unset, unknown),
        );
    } else {
        *unknown = (*unknown).union(DynamicNameBarrier::OPAQUE_SCRIPT);
    }
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

/// Command store and exact availability for variable-presence source queries.
#[derive(Clone, Copy)]
pub struct ExistenceMetadataInputs<'a> {
    /// Actual graph command store.
    pub registry: &'a CommandRegistry,
    /// Supplied availability; absence and foreign stores refuse folding.
    pub context: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
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
    let metadata = ExistenceMetadataInputs {
        registry,
        context: registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile)
            .map(Into::into),
    };
    existence_constant_branches_with_ssa_with_metadata_context(cfg, frame, metadata, config, ssa)
}

fn known_existence_query(
    var: &str,
    kind: crate::existence_query::ExistenceKind,
    frame: ExistenceFrame<'_>,
    (defined, unset): (&FxHashSet<String>, &FxHashSet<String>),
    aliased: &HashSet<String>,
    dynamic_names: crate::dynamic_names::DynamicNameBarrier,
) -> Option<bool> {
    let unknown_writes = dynamic_names.writes;
    Some(if let Some(base) = array_element_base(var) {
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
            || unknown_writes
            || defined
                .iter()
                .any(|d| d == base || d.strip_prefix(base).is_some_and(|r| r.starts_with('(')))
        {
            return None;
        }
        false
    } else {
        // Namespaced globals may be populated outside the function's
        // view — only fold simple local names.
        if var.is_empty() || !var.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return None;
        }
        // A scope-alias local's existence tracks the linked variable —
        // never fold it (see the `aliased` collection above).
        if aliased.contains(var) {
            return None;
        }
        if frame.params.iter().any(|p| p.as_str() == var) {
            // A literal `unset x` already blocks this; a computed
            // `unset $n` can name the parameter just as well
            // (tclsh 9.0.4 / 8.6.14: `proc f {p n} {unset $n; info exists p}`
            // → `0` for `f hello p`), so the barrier blocks it too.
            if unset.contains(var) || dynamic_names.destroys {
                return None;
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
        } else if !defined.contains(var) {
            // `set $switch {}` may have defined exactly this name — the
            // argparse idiom.
            if unknown_writes {
                return None;
            }
            false
        } else {
            return None;
        }
    })
}

/// Existence query under the supplied actual availability generation.
#[must_use]
pub fn existence_constant_branches_with_ssa_with_metadata_context(
    cfg: &CfgFunction,
    frame: ExistenceFrame<'_>,
    metadata: ExistenceMetadataInputs<'_>,
    config: tcl_lexer::LexerConfig,
    ssa: &SsaFunction,
) -> Vec<ConstantBranch> {
    let registry = metadata.registry;
    let Some(context) = metadata
        .context
        .filter(|context| context.matches_registry(registry))
    else {
        return Vec::new();
    };
    let Some(points) = &ssa.point_contexts else {
        return existence_constant_branches_with_metadata_context(
            cfg,
            frame,
            metadata,
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
            let (query, context) = crate::existence_query::in_expr_at_with_metadata_context(
                condition,
                *base,
                tokens,
                registry,
                config,
                Some(context),
            )?;
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
                kind: super::BranchFactKind::Applied,
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
fn array_element_base(var: &str) -> Option<&str> {
    let (base, _) = tcl_syntax::naming::split_element_ref(var)?;
    if base.is_empty() {
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
    let metadata = ExistenceMetadataInputs {
        registry,
        context: registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile)
            .map(Into::into),
    };
    existence_constant_branches_with_metadata_context(cfg, frame, metadata, dynamic_names, config)
}

/// Existence query under the supplied actual availability generation.
#[must_use]
pub fn existence_constant_branches_with_metadata_context(
    cfg: &CfgFunction,
    frame: ExistenceFrame<'_>,
    metadata: ExistenceMetadataInputs<'_>,
    dynamic_names: crate::dynamic_names::DynamicNameBarrier,
    config: tcl_lexer::LexerConfig,
) -> Vec<ConstantBranch> {
    let registry = metadata.registry;
    let Some(context) = metadata
        .context
        .filter(|context| context.matches_registry(registry))
    else {
        return Vec::new();
    };
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
    let (defined, unset, call_names) = scan_defined_and_unset(cfg, metadata);
    let dynamic_names = dynamic_names.union(call_names);
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
    let mut aliased = crate::optimiser::elimination::scan_scope_aliases_with_metadata_context(
        cfg,
        registry,
        Some(context),
    );
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
                context.context().authoring_query(),
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
            crate::existence_query::in_expr_with_metadata_context(
                condition,
                registry,
                config,
                Some(context),
            )
        else {
            continue;
        };
        let Some(exists) = known_existence_query(
            &var,
            kind,
            frame,
            (&defined, &unset),
            &aliased,
            dynamic_names,
        ) else {
            continue;
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
            kind: super::BranchFactKind::Applied,
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

#[cfg(test)]
pub(super) fn evaluate_fixture_def<'a, S: std::hash::BuildHasher>(
    stmt_ssa: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    ssa: impl Into<SsaSourceView<'a>>,
    policy: FoldPolicy,
    config: tcl_lexer::LexerConfig,
) -> LatticeValue {
    evaluate_fixture_def_with_folds(stmt_ssa, values, ssa, policy, None, config)
}

#[cfg(test)]
pub(super) fn evaluate_fixture_def_with_folds<'a, S: std::hash::BuildHasher>(
    stmt_ssa: &SsaStatement,
    values: &HashMap<ValueKey, LatticeValue, S>,
    ssa: impl Into<SsaSourceView<'a>>,
    policy: FoldPolicy,
    folds: Option<BuiltinFoldInputs<'_>>,
    config: tcl_lexer::LexerConfig,
) -> LatticeValue {
    evaluate_def_with_math(
        stmt_ssa,
        values,
        ssa.into(),
        policy,
        folds,
        None,
        Some(config),
    )
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
                ..SccpResult::default()
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
    value: Option<&FoldedExpressionContents>,
) -> bool {
    if analysis || matches!(point, Some(ExpressionEvaluationPoint::Branch { .. })) {
        return false;
    }
    // ConstValue retains contents only. A retained result instruction may also
    // carry normalisation, allocation or pool-reuse semantics that this
    // projection cannot preserve. Consumed branch values have no output object.
    preparation.is_some_and(|preparation| {
        match value {
            Some(FoldedExpressionContents::Numeric(value)) => {
                crate::expression_rewrite::contents_only_result_equivalence(
                    &preparation.witness,
                    value,
                )
            }
            _ => crate::expression_rewrite::expression_result_protocol_equivalence(
                &preparation.witness,
                None,
            ),
        }
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
    if context.is_some_and(|context| context.bindings.is_positioned()) && preparation.is_none() {
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
    if retained_result_needs_native_protocol(preparation, point, analysis_purpose, value.as_ref()) {
        return None;
    }
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
    reference_config: Option<tcl_lexer::LexerConfig>,
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
                    fold_assign_value(
                        value,
                        &stmt_ssa.uses,
                        values,
                        ssa,
                        policy,
                        folds,
                        reference_config,
                    )
                })
        }
        Statement::Call {
            command,
            args,
            defs,
            tokens,
            ..
        } if single_iteration_binding(command, args, defs, tokens.as_ref()) => {
            fold_iteration_value(
                &args[0],
                stmt_ssa,
                values,
                ssa,
                policy,
                folds,
                reference_config,
            )
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
                    } else if let Some(amount) = resolve_simple_var_ref(
                        trimmed,
                        &stmt_ssa.uses,
                        values,
                        ssa,
                        reference_config,
                    ) {
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
    source_metadata_input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
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
    let inputs = folds?;
    let registry = inputs.registry;
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
        registry,
        inputs.source_metadata_input?,
    )?;
    let tokens = statement.statement.tokens()?;
    let invocation = crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
        registry,
        Some(metadata),
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
            source_metadata_input: inputs.source_metadata_input,
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
    if let Some(value) =
        fold_original_substitution_value(tokens, site, &statement.uses, values, source, inputs)
    {
        return Some(LatticeValue::Const(parse_literal_value(&value)));
    }
    if inputs
        .math?
        .dependencies
        .is_some_and(|ledger| ledger.purpose == SolverPurpose::SemanticAnalysis)
        && let Some(value) = fold_prepared_invocation_expression(spelling, site, inputs)
    {
        return Some(value);
    }
    let input = inputs.source_metadata_input?;
    let context = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
        inputs.registry,
        input,
    )?;
    let mut parser = tcl_syntax::expr::parser::ExprParseContext::for_profile(input.unit_profile());
    parser.lexer_grammar = input.lexer_config().grammar_over(parser.lexer_grammar);
    let expressions = crate::word_subst::lifted_source_expressions_with_metadata_context(
        Some(tokens),
        inputs.registry,
        Some(context),
        input.lexer_config(),
        parser,
    );
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

/// A selected original substitution and its complete child inventory.
/// Missing actual input or handler identity never falls back to written names.
fn fold_original_substitution_value<S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
    tokens: &crate::ir::CommandTokens,
    site: &crate::ir::SourceSite,
    uses: &HashMap<Symbol, crate::ssa::Version, S1>,
    values: &HashMap<ValueKey, LatticeValue, S2>,
    source: SsaSourceView<'_>,
    inputs: RetainedExpressionFoldInputs<'_>,
) -> Option<String> {
    let input = inputs.source_metadata_input?;
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
        inputs.registry,
        input,
    )?;
    let calls = crate::word_subst::checked_lifted_calls(tokens, input.lexer_config().nested())?;
    let mut selected = calls.iter().filter(|call| call.span == site.span);
    let call = selected.next()?;
    if selected.next().is_some() {
        return None;
    }
    let lookup = |name: &str| lattice_const_text(name, uses, values, source, inputs.policy);
    let trust = |_name: &str| false;
    crate::const_subst::ConstSubstCtx {
        registry: inputs.registry,
        resolution_namespace: "::",
        namespace_context: None,
        version: input.unit_profile().const_fold_version(),
        defining_class: None,
        trusts: &trust,
        lookup_var: &lookup,
        script_encoding: tcl_registry::value_transfer::ScriptEncoding::Auto,
    }
    .fold_retained_call_with_metadata_context(call, &calls, metadata)
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
    reference_config: Option<tcl_lexer::LexerConfig>,
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
                reference_config,
            )
        })
        .or_else(|| {
            // `foreach v [list a b c]` — fold the command substitution
            // to a constant list string, then split into elements.
            let arg = argument.trim();
            if arg.starts_with('[')
                && arg.ends_with(']')
                && let Some(LatticeValue::Const(ConstValue::String(s))) = try_fold_cmd_subst(
                    arg,
                    &statement.uses,
                    values,
                    source,
                    policy,
                    folds,
                    reference_config,
                )
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
    let inputs = folds?;
    let registry = inputs.registry;
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
        registry,
        inputs.source_metadata_input?,
    )?;
    let binding = tokens.source_binding.as_ref()?;
    let invocation = crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
        registry,
        Some(metadata),
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
            source_metadata_input: inputs.source_metadata_input,
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
        Some(inputs.source_metadata_input?.lexer_config()),
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
            // value-transfer-ok: irreducible — the tokenless legacy caller has
            // no retained invocation to ask; tokens consult the synthetic marker.
            // registry-axis-ok: command — tokenless legacy fold; the tokened path asks the registry; until the tokenless SCCP caller is retired
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
    reference_config: Option<tcl_lexer::LexerConfig>,
) -> Option<LatticeValue> {
    let ssa = ssa.into();
    let name = tcl_syntax::naming::whole_scalar_variable_reference_name(text, reference_config?)?;
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
    /// The document configuration a `Raw` whole-variable operand (the
    /// flattened `switch` subject) is read under.
    reference_config: Option<tcl_lexer::LexerConfig>,
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
    // The flattened `switch` compares its subject as a `Raw` whole-variable
    // operand, which the evaluator reads only as that variable (#2375).
    let resolved = fold
        .reference_config
        .and_then(|config| crate::sccp::with_whole_variable_operands(condition, config));
    let condition = resolved.as_ref().unwrap_or(condition);
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
/// pre-loop block's exit under the retained CFG metadata and fold Registry.
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
    let condition_base = match &node.statement {
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
            &node.statement,
            &start_env,
            loop_math,
            fold,
        );
    }
    let summarised = crate::static_loops::summarise_for_statement_with_source_dependencies(
        &node.statement,
        &start_env,
        crate::static_loops::DEFAULT_MAX_STATIC_LOOP_ITERS,
        policy,
        Some(loop_math),
        crate::static_loops::StaticLoopSource::new(fold.registry, &cfg.metadata_context),
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
    let analysis = crate::static_loops::analyse_for_statement_with_source_dependencies(
        statement,
        initial,
        crate::static_loops::DEFAULT_MAX_STATIC_LOOP_ITERS,
        fold.policy,
        Some(bindings),
        crate::static_loops::StaticLoopSource::new(fold.registry, &cfg.metadata_context),
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
    reference_config: Option<tcl_lexer::LexerConfig>,
) -> Option<Vec<String>>
where
    S1: std::hash::BuildHasher,
    S2: std::hash::BuildHasher,
{
    let ssa = ssa.into();
    let stripped = list_text.trim();
    let name =
        tcl_syntax::naming::whole_scalar_variable_reference_name(stripped, reference_config?)?;
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
    reference_config: Option<tcl_lexer::LexerConfig>,
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
    if let Some(tokens) = ssa.source_tokens() {
        let Some(inputs) = folds else {
            return LatticeValue::Overdefined;
        };
        let Some(metadata) = inputs.source_metadata_input.and_then(|input| {
            crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
                inputs.registry,
                input,
            )
        }) else {
            return LatticeValue::Overdefined;
        };
        let Some(invocation) =
            crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
                inputs.registry,
                Some(metadata),
                tokens,
            )
        else {
            return LatticeValue::Overdefined;
        };
        let Some((_, word)) = tokens.source_binding.as_ref().and_then(|binding| {
            invocation.stored_value_operand(&binding.variable_context, inputs.registry)
        }) else {
            return LatticeValue::Overdefined;
        };
        let Some((_, site)) = word.sole_command_substitution() else {
            return LatticeValue::Overdefined;
        };
        return fold_original_substitution_value(
            tokens,
            site,
            uses,
            values,
            ssa,
            RetainedExpressionFoldInputs {
                source_metadata_input: inputs.source_metadata_input,
                registry: inputs.registry,
                policy,
                math: None,
            },
        )
        .map_or(LatticeValue::Overdefined, |value| {
            LatticeValue::Const(parse_literal_value(&value))
        });
    }
    // Legacy tokenless callers retain the unique-binding compatibility view.
    if let Some(resolved) = resolve_simple_var_ref(stripped, uses, values, ssa, reference_config) {
        return resolved;
    }
    // Command substitution.
    if stripped.starts_with('[') && stripped.ends_with(']') {
        if let Some(lv) =
            try_fold_cmd_subst(stripped, uses, values, ssa, policy, folds, reference_config)
        {
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
                script_encoding: tcl_registry::value_transfer::ScriptEncoding::Auto,
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
    reference_config: Option<tcl_lexer::LexerConfig>,
) -> Option<String> {
    let ssa = ssa.into();
    let arg = arg.trim();
    if arg.starts_with('$') {
        let name =
            tcl_syntax::naming::whole_scalar_variable_reference_name(arg, reference_config?)?;
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
    reference_config: Option<tcl_lexer::LexerConfig>,
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
        && let Some(folded) = crate::codegen_helpers::fold_list_cmd(value, policy.word_rules)
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
        && let Some(folded) = crate::codegen_helpers::try_format_fold(value, escapes)
    {
        return Some(LatticeValue::Const(ConstValue::String(folded)));
    }

    let inner = value.strip_prefix('[')?.strip_suffix(']')?;
    let (cmd, rest) = split_head(inner);
    if !trusted(cmd) {
        return None;
    }

    // `[llength LIST]` with a literal or lattice-resolvable list.
    // value-transfer-ok: irreducible — tokenless legacy fold; a retained
    // invocation folds through the registry's const_fold instead.
    // registry-axis-ok: command — tokenless legacy fold; the tokened path asks the registry; until the tokenless SCCP caller is retired
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
        if let Some(items) = resolve_foreach_list_via_lattice(
            arg,
            uses,
            values,
            ssa,
            policy.word_rules,
            reference_config,
        ) {
            let n = i64::try_from(items.len()).unwrap_or(i64::MAX);
            return Some(LatticeValue::Const(ConstValue::Int(n)));
        }
        return None;
    }

    // `[string length OPERAND]` where OPERAND resolves to a constant
    // string — a literal word, or a `$var` whose lattice value is known.
    // (Counting the chars of the *unresolved* operand text would mis-fold
    // `string length $s` to the length of "$s".)
    // value-transfer-ok: irreducible — tokenless legacy fold, as for llength.
    // registry-axis-ok: command — tokenless legacy fold; the tokened path asks the registry; until the tokenless SCCP caller is retired
    if cmd == "string" {
        if let Some(after_cmd) = rest {
            let (sub, sub_rest) = split_head(after_cmd.trim());
            // value-transfer-ok: irreducible — tokenless legacy fold, as above.
            // registry-axis-ok: command — tokenless legacy fold; the tokened path asks the registry; until the tokenless SCCP caller is retired
            if sub == "length"
                && let Some(raw) = sub_rest
                && let Some(s) =
                    resolve_const_string(raw.trim(), uses, values, ssa, policy, reference_config)
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
    // value-transfer-ok: irreducible — tokenless legacy fold, as for llength.
    // registry-axis-ok: command — tokenless legacy fold; the tokened path asks the registry; until the tokenless SCCP caller is retired
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
mod tests;
