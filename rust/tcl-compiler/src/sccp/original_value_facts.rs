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

#[cfg(test)]
use rustc_hash::FxHashSet;
use tcl_registry::CommandRegistry;

#[cfg(test)]
use crate::analyses::MAX_CONSTSET_SIZE;
use crate::analyses::{ConstValue, LatticeValue};
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::expr_ast::ExprNode;
use crate::ir::Statement;
use crate::math_function_binding::ExpressionMathBindings;
use crate::ssa::{SsaFunction, SsaSourceView, Symbol, ValueKey};
use crate::tcl_expr_eval::FoldPolicy;
#[cfg(test)]
use crate::tcl_expr_eval::{Env, EnvValue, TclValue};

use super::{BuiltinFoldInputs, ConstantBranch, FoldTrust, SccpResult, TraceInputs};
#[cfg(test)]
use super::{cfg_order, compute_predecessors, join, sccp_process_phis, set_value};

mod assignment_folds;
mod block_transfer;
mod branch_decisions;
mod definition_values;
mod existence_branches;
mod math_expressions;
mod value_solver;

#[cfg(test)]
use super::{extract_foreach_elements, parse_literal_value, resolve_foreach_list_via_lattice};
#[cfg(test)]
use assignment_folds::{resolve_const_string, split_head, strip_one_level};
#[cfg(test)]
use block_transfer::sccp_process_statements;
pub use branch_decisions::evaluate_branch;
use branch_decisions::materialised_branch_bindings;
#[cfg(test)]
use definition_values::resolve_simple_var_ref;
#[cfg(test)]
pub(super) use definition_values::{evaluate_fixture_def, evaluate_fixture_def_with_folds};
#[cfg(test)]
use existence_branches::scan_defined_and_unset;
pub use existence_branches::{
    ExistenceFrame, ExistenceMetadataInputs, existence_constant_branches,
    existence_constant_branches_with_metadata_context, existence_constant_branches_with_ssa,
    existence_constant_branches_with_ssa_with_metadata_context,
};
use math_expressions::{env_from_uses, fold_math_expression};
#[cfg(test)]
use value_solver::branch_deferrable;
#[cfg(test)]
use value_solver::compatibility_escaping;
use value_solver::solve_value_facts;

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
    origin: Option<&'a tcl_names::origin::ExecutedScriptSource>,
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

#[derive(Clone, Copy)]
struct RetainedExpressionFoldInputs<'a> {
    source_metadata_input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
    registry: &'a CommandRegistry,
    policy: FoldPolicy,
    math: Option<MathFoldContext<'a>>,
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

#[cfg(test)]
mod tests;
