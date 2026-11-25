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

//! Folding an original expression with its math-function dependencies retained, and the evaluation environment read from the lattice.

use super::super::{ConstantBranch, SccpResult};
use super::definition_values::source_read_key;
use super::{
    ExpressionAnalysis, ExpressionEvaluationPoint, MathDependencies, MathFoldContext,
    SolvedValueFacts, SolverPurpose,
};
use crate::analyses::{ConstValue, LatticeValue};
use crate::cfg::BlockId;
use crate::expr_ast::ExprNode;
use crate::ssa::{SsaSourceView, Symbol, ValueKey};
use crate::tcl_expr_eval::{Env, FoldPolicy, TclValue, const_to_env_value};
use std::collections::{HashMap, HashSet};
use tcl_registry::value_transfer::ExactValue;

impl MathDependencies {
    pub(super) fn for_parameters(
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

    pub(super) fn finish(
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

    pub(super) fn retain(
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

fn math_dependencies_compatible(
    previous: &crate::command_binding::SourceMathInvocation,
    current: &crate::command_binding::SourceMathInvocation,
) -> bool {
    crate::math_function_binding::native_math_prerequisites_compatible(
        previous.fixed_prerequisite(),
        current.fixed_prerequisite(),
    )
}

pub(super) enum FoldedExpressionContents {
    Numeric(TclValue),
    Exact(ExactValue<String>),
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
    pub(super) fn into_const(self) -> ConstValue {
        match self {
            Self::Numeric(value) => tcl_value_to_const(value),
            Self::Exact(value) => ConstValue::String(value.bytes),
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

pub(super) fn fold_math_expression(
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
        Some(NativeExpressionResultDependency::SelectedOperand { existing_value, .. }) => {
            existing_value.clone().map(FoldedExpressionContents::Exact)
        }
        Some(NativeExpressionResultDependency::StringResult { value }) => {
            Some(FoldedExpressionContents::Exact(value.clone()))
        }
        None => Some(FoldedExpressionContents::Numeric(evaluation.value.clone())),
    }
}

/// Build a [`tcl_expr_eval::Env`] from a `{symbol → version}` map
/// and the current lattice. Only entries whose lattice value is
/// a single [`LatticeValue::Const`] are bound; anything else
/// leaves the variable unbound so the evaluator returns `None`.
pub(super) fn env_from_uses<'a, S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
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
pub(super) fn env_from_uses_numeric<'a, S1: std::hash::BuildHasher, S2: std::hash::BuildHasher>(
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
