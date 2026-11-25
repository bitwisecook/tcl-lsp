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

//! Reached preparation proofs for exact expression source objects.

use super::{
    Arc, CommandAllocationSite, ExecutedScriptSource, ModuleCommandBindings, SourceCommandBindings,
    SourceExecutionContext, SourceOutcomes,
};
use tcl_registry::runtime_expr_validation::{
    ExpressionPreparationProof, NativeExpressionScriptPreparation, PreparedExpressionWitness,
    prepare_expression_witness_with_script_compiler,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum ExpressionCoercionPhase {
    CompilerPreparation,
    RuntimeOperands,
}

#[derive(Debug)]
struct CurrentExpressionReadCapture {
    invocation: CommandAllocationSite,
    source: Arc<super::SourceOriginId>,
    frame: crate::var_resolve::VariableExecutionFrame,
    reads: Vec<super::SourceVariableAccess>,
    incompatible: bool,
}

/// A synchronous expression evaluation owns its own original read events.
/// Retained source inventories still join every represented invocation.
#[derive(Debug, Default)]
pub(super) struct CurrentExpressionReadCaptures(Vec<CurrentExpressionReadCapture>);

impl Clone for CurrentExpressionReadCaptures {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl SourceCommandBindings {
    fn begin_expression_read_capture(
        &mut self,
        proof: &SourceExpressionPreparation,
        frame: &crate::var_resolve::VariableExecutionFrame,
    ) -> usize {
        let depth = self.current_expression_reads.0.len();
        self.current_expression_reads
            .0
            .push(CurrentExpressionReadCapture {
                invocation: proof.invocation.clone(),
                source: Arc::clone(&proof.source.origin),
                frame: frame.clone(),
                reads: Vec::new(),
                incompatible: false,
            });
        depth
    }

    pub(super) fn capture_current_expression_read(
        &mut self,
        origin: &Arc<super::SourceOriginId>,
        frame: &crate::var_resolve::VariableExecutionFrame,
        read: &super::SourceVariableAccess,
    ) {
        let Some(capture) = self.current_expression_reads.0.last_mut() else {
            return;
        };
        if !matches!(&read.owner, super::SourceVariableEvaluationOwner::NativeExpression {
            invocation, ..
        } if invocation == &capture.invocation)
        {
            return;
        }
        if origin != &capture.source || frame != &capture.frame {
            capture.incompatible = true;
            return;
        }
        if !capture.reads.contains(read) {
            capture.reads.push(read.clone());
        }
    }

    fn finish_expression_read_capture(
        &mut self,
        depth: usize,
        proof: &SourceExpressionPreparation,
    ) -> Vec<super::SourceVariableAccess> {
        if self.current_expression_reads.0.len() != depth + 1 {
            self.current_expression_reads.0.truncate(depth);
            return Vec::new();
        }
        let Some(capture) = self.current_expression_reads.0.pop() else {
            return Vec::new();
        };
        if capture.incompatible || capture.invocation != proof.invocation {
            return Vec::new();
        }
        capture.reads
    }
}

/// Operand evaluation carries effects and abrupt results. A whole expression's
/// normal result belongs to its selected native result producer independently.
fn discard_normal_operand_results(outcomes: &mut SourceOutcomes) {
    outcomes.normal_completion = None;
    outcomes.normal_value = None;
    outcomes.normal_name_value = None;
    outcomes.normal_representation = None;
    outcomes.normal_object = None;
    outcomes.normal_method_prefix = None;
    outcomes.normal_rhs_read = None;
}

/// A sharing footprint, not an inventory of executed reads. Preparation can
/// touch literal/pool objects; variable objects belong to reached execution.
fn expression_coercion_values(
    tree: &crate::expr_ast::ExprNode,
    variables: &crate::var_resolve::ResolveContext,
    registry: &tcl_registry::CommandRegistry,
    policy: crate::tcl_expr_eval::FoldPolicy,
    phase: ExpressionCoercionPhase,
) -> Option<Vec<String>> {
    use crate::expr_ast::ExprNode;
    let mut pending = vec![tree];
    let mut values = Vec::new();
    let numeric_operands = crate::native_numeric::numeric_tree(tree, true, |_| true);
    // Constant compiler results can be pooled under bytes absent from the
    // original literals. Evaluate each maximal constant subtree once.
    let native = policy.preparation_context()?.native_syntax;
    for subtree in
        tcl_registry::runtime_expr_validation::native_expression_pooled_subtrees(tree, native)?
    {
        if phase == ExpressionCoercionPhase::RuntimeOperands {
            continue;
        }
        let number = crate::tcl_expr_eval::eval_tcl_expr_with_policy(
            subtree,
            &crate::tcl_expr_eval::Env::new(),
            policy,
        )?;
        values.push(crate::tcl_expr_eval::format_tcl_value_with_policy(
            &number, policy,
        )?);
    }
    while let Some(node) = pending.pop() {
        match node {
            ExprNode::Literal { text, .. } => {
                values.push(text.clone());
                // Compiler literal pools may store canonical numeric bytes
                // rather than the original boolean/hexadecimal spelling.
                let number = crate::tcl_expr_eval::parse_literal_in(text, policy.numbers?)?;
                values.push(crate::tcl_expr_eval::format_tcl_value_with_policy(
                    &number, policy,
                )?);
            }
            ExprNode::Var { text, .. } => {
                if phase == ExpressionCoercionPhase::CompilerPreparation {
                    continue;
                }
                let place = crate::var_resolve::resolve_substitution_access(
                    text,
                    variables,
                    registry,
                    tcl_registry::TraceOperation::Read,
                );
                if !numeric_operands
                    || !variables.contents_already_native_numeric_at(&place, registry)
                {
                    values.push(variables.literal_contents_at(&place, registry)?.to_owned());
                }
            }
            ExprNode::Binary { left, right, .. } => pending.extend([left.as_ref(), right.as_ref()]),
            ExprNode::Unary { operand, .. } => pending.push(operand),
            ExprNode::Call { args, .. }
                if phase == ExpressionCoercionPhase::CompilerPreparation =>
            {
                // Function implementations are reached only after runtime
                // operands. Preparation still owns constant operand pools.
                pending.extend(args);
            }
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
            ExprNode::String { text, .. } => {
                values.push(tcl_syntax::expr::fixed_string_operand(text)?.to_owned());
            }
            // A substitution's result does not exist while the compiler pools
            // operands; its own commands are compiled separately.
            ExprNode::Command { .. } if phase == ExpressionCoercionPhase::CompilerPreparation => {}
            ExprNode::CompiledWord { .. }
            | ExprNode::Command { .. }
            | ExprNode::Call { .. }
            | ExprNode::Raw { .. } => return None,
        }
    }
    Some(values)
}

fn expression_reads_already_numeric(
    tree: &crate::expr_ast::ExprNode,
    variables: &crate::var_resolve::ResolveContext,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    crate::native_numeric::numeric_tree(tree, false, |node| {
        let crate::expr_ast::ExprNode::Var { text, .. } = node else {
            return false;
        };
        let place = crate::var_resolve::resolve_substitution_access(
            text,
            variables,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        variables.contents_already_native_numeric_at(&place, registry)
    })
}

fn expression_has_only_numeric_conversions(
    tree: &crate::expr_ast::ExprNode,
    policy: crate::tcl_expr_eval::FoldPolicy,
) -> bool {
    let Some(numbers) = policy.numbers else {
        return false;
    };
    if !crate::native_numeric::numeric_tree(tree, true, |_| true) {
        return false;
    }
    let comparison_fallback = policy.preparation_context().and_then(|context| {
        tcl_registry::runtime_expr_validation::native_numeric_comparison_fallback(
            context.native_syntax,
        )
    });
    let preserves_comparisons = comparison_fallback
        == Some(tcl_registry::runtime_expr_validation::NativeEqualityFallbackProtocol::PreservesNumericRepresentation);
    let mut pending = vec![tree];
    while let Some(node) = pending.pop() {
        use crate::expr_ast::ExprNode;
        match node {
            ExprNode::Literal { text, .. } => {
                if crate::tcl_expr_eval::parse_literal_in(text, numbers).is_none() {
                    return false;
                }
            }
            ExprNode::Unary { op, operand } => {
                if !preserves_comparisons
                    && matches!(
                        op,
                        crate::expr_ast::UnaryOp::Not | crate::expr_ast::UnaryOp::WordNot
                    )
                {
                    return false;
                }
                pending.push(operand);
            }
            ExprNode::Binary { op, left, right } => {
                use crate::expr_ast::BinOp;
                if !preserves_comparisons
                    && matches!(
                        op,
                        BinOp::Eq
                            | BinOp::Ne
                            | BinOp::Lt
                            | BinOp::Le
                            | BinOp::Gt
                            | BinOp::Ge
                            | BinOp::And
                            | BinOp::Or
                    )
                {
                    return false;
                }
                pending.extend([left.as_ref(), right.as_ref()]);
            }
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                if !preserves_comparisons {
                    return false;
                }
                pending.extend([
                    condition.as_ref(),
                    true_branch.as_ref(),
                    false_branch.as_ref(),
                ]);
            }
            ExprNode::Var { .. } => {}
            _ => return false,
        }
    }
    true
}

/// Captured before operand evaluation, so a later callback cannot replace
/// an old operand's byte footprint with its new same-spelling variable value.
pub(super) struct RuntimeExpressionCoercion {
    values: Option<Vec<String>>,
    numeric_only: bool,
}

impl RuntimeExpressionCoercion {
    pub(super) fn capture(
        tree: &crate::expr_ast::ExprNode,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Self> {
        capture_expression_coercion(
            tree,
            state,
            context,
            ExpressionCoercionPhase::RuntimeOperands,
        )
    }

    pub(super) fn apply(self, state: &mut ModuleCommandBindings) {
        let variables = Arc::make_mut(&mut state.source_variables);
        if self.numeric_only {
            variables.invalidate_numeric_conversion_representations(self.values.as_deref());
        } else {
            variables.coerce_representations_for_values(self.values.as_deref());
        }
    }
}

fn capture_expression_coercion(
    tree: &crate::expr_ast::ExprNode,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    phase: ExpressionCoercionPhase,
) -> Option<RuntimeExpressionCoercion> {
    if expression_reads_already_numeric(tree, &state.source_variables, context.registry) {
        return None;
    }
    let policy = crate::tcl_expr_eval::FoldPolicy::for_retained_entry(
        context.registry,
        state.baseline.dialect,
        &context.config,
    );
    let values = expression_coercion_values(
        tree,
        &state.source_variables,
        context.registry,
        policy,
        phase,
    );
    if values.as_ref().is_some_and(Vec::is_empty) {
        return None;
    }
    Some(RuntimeExpressionCoercion {
        values,
        numeric_only: expression_has_only_numeric_conversions(tree, policy),
    })
}

fn invalidate_preparation_representations(
    tree: &crate::expr_ast::ExprNode,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) {
    if let Some(coercion) = capture_expression_coercion(
        tree,
        state,
        context,
        ExpressionCoercionPhase::CompilerPreparation,
    ) {
        coercion.apply(state);
    }
}

pub(super) fn root_result_coercion(
    tree: &crate::expr_ast::ExprNode,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<RuntimeExpressionCoercion> {
    (!matches!(
        tree,
        crate::expr_ast::ExprNode::Binary { .. }
            | crate::expr_ast::ExprNode::Unary { .. }
            | crate::expr_ast::ExprNode::Ternary { .. }
            | crate::expr_ast::ExprNode::Call { .. }
    ))
    .then(|| RuntimeExpressionCoercion::capture(tree, state, context))
    .flatten()
}

/// Integer operand contents are a conversion protocol, not an intrep/type
/// label. Every original occurrence must retain a closed live read, and
/// literal operands require the actual unchanged stock pool.
fn expression_has_integer_inputs(
    preparation: &SourceExpressionPreparation,
    state: &ModuleCommandBindings,
    reads: &[super::SourceVariableAccess],
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    use crate::expr_ast::ExprNode;
    use tcl_syntax::expr::{BinOp, UnaryOp};
    let Some(dialect) = state.baseline.dialect else {
        return false;
    };
    if dialect.arithmetic() != Some(tcl_dialect::NativeArithmetic::TclBignum) {
        return false;
    }
    let mut pending = vec![preparation.witness.tree()];
    while let Some(node) = pending.pop() {
        match node {
            ExprNode::Literal { text, .. } => {
                if !state.ordinary_literal_pool.as_ref().is_some_and(
                    super::literal_object_pool::SourceOrdinaryLiteralPool::authored_objects,
                ) || !matches!(
                    tcl_syntax::number::parse_whole_with(
                        text,
                        tcl_syntax::number::ParseFlags::for_syntax(dialect.numbers)
                    ),
                    Some(
                        tcl_syntax::number::Number::Int(_) | tcl_syntax::number::Number::Big { .. }
                    )
                ) {
                    return false;
                }
            }
            ExprNode::Var { .. } => {
                let Some((_, source)) = preparation.original_variable_source(node) else {
                    return false;
                };
                let mut matching = reads.iter().filter(|access| access.source == source
                    && matches!(&access.owner, super::SourceVariableEvaluationOwner::NativeExpression { invocation, .. }
                        if invocation == &preparation.invocation));
                let Some(read) = matching.next() else {
                    return false;
                };
                if matching.next().is_some()
                    || read.context_residual() != super::SourceVariableReadResidual::Closed
                    || read.context_alternatives().is_empty()
                    || !read.context_alternatives().iter().all(|context| {
                        let place = read.place_in_context(context, registry);
                        context
                            .contents_integer_increment_conversion_at(&place, registry)
                            .is_some()
                    })
                {
                    return false;
                }
            }
            ExprNode::Binary {
                op:
                    BinOp::Add
                    | BinOp::Sub
                    | BinOp::Mul
                    | BinOp::Div
                    | BinOp::LShift
                    | BinOp::RShift
                    | BinOp::BitAnd
                    | BinOp::BitOr
                    | BinOp::BitXor,
                left,
                right,
            } => {
                pending.extend([left.as_ref(), right.as_ref()]);
            }
            ExprNode::Unary {
                op: UnaryOp::Pos | UnaryOp::Neg | UnaryOp::BitNot,
                operand,
            } => {
                pending.push(operand);
            }
            _ => return false,
        }
    }
    true
}

/// Only integer operations without a value-dependent failure can certify
/// complete evaluation. Normal-result type inference has a wider domain.
fn integer_expression_completes_normally(
    preparation: &SourceExpressionPreparation,
    state: &ModuleCommandBindings,
    reads: &[super::SourceVariableAccess],
    context: SourceExecutionContext<'_>,
) -> bool {
    use crate::expr_ast::ExprNode;
    use tcl_syntax::expr::{BinOp, UnaryOp};
    let Some(dialect) = state.baseline.dialect else {
        return false;
    };
    let policy = crate::tcl_expr_eval::FoldPolicy::for_retained_entry(
        context.registry,
        Some(dialect),
        &context.config,
    );
    if tcl_registry::native_numeric_conversion::NativeStockIntegerContentsProtocol::select(dialect)
        .is_none()
        || policy.preparation_context().as_ref() != Some(preparation.witness.context())
        || !preparation.has_closed_script_compilation()
        || !expression_has_integer_inputs(preparation, state, reads, context.registry)
    {
        return false;
    }
    let mut pending = vec![preparation.witness.tree()];
    while let Some(node) = pending.pop() {
        match node {
            ExprNode::Literal { .. } | ExprNode::Var { .. } => {}
            ExprNode::Binary {
                op:
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor,
                left,
                right,
            } => pending.extend([left.as_ref(), right.as_ref()]),
            ExprNode::Unary {
                op: UnaryOp::Pos | UnaryOp::Neg | UnaryOp::BitNot,
                operand,
            } => pending.push(operand),
            _ => return false,
        }
    }
    true
}

fn original_expression_read<'a>(
    preparation: &SourceExpressionPreparation,
    node: &crate::expr_ast::ExprNode,
    reads: &'a [super::SourceVariableAccess],
) -> Option<&'a super::SourceVariableAccess> {
    let (_, source) = preparation.original_variable_source(node)?;
    let mut candidates = reads.iter().filter(|access| {
        access.source == source
            && matches!(&access.owner, super::SourceVariableEvaluationOwner::NativeExpression { invocation, .. } if invocation == &preparation.invocation)
    });
    let read = candidates.next()?;
    candidates.next().is_none().then_some(read)
}

fn expression_evaluation_completes_normally(
    evaluation: &crate::tcl_expr_eval::FoldEvaluation,
    outcomes: &SourceOutcomes,
    preparation: &SourceExpressionPreparation,
    state: &ModuleCommandBindings,
    reads: &[super::SourceVariableAccess],
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    state
        .ordinary_literal_pool
        .as_ref()
        .is_some_and(super::literal_object_pool::SourceOrdinaryLiteralPool::authored_objects)
        && !state.has_opaque_domain()
        && preparation.has_closed_script_compilation()
        && evaluation.native_value_effects_are_proved()
        && reads.iter().all(|read| {
            read.context_residual() == super::SourceVariableReadResidual::Closed
                && !read.context_alternatives().is_empty()
                && read.context_alternatives().iter().all(|variables| {
                    let place = read.place_in_context(variables, registry);
                    !place.observed && variables.read_produces_value(&place, registry)
                })
        })
        && outcomes.abrupt.iter().all(|(route, _)| {
            *route
                == tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                    tcl_registry::completion::CompletionCode::Error,
                )
        })
}

fn native_numeric_result(
    outcomes: &mut SourceOutcomes,
    preparation: &SourceExpressionPreparation,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    reads: &[super::SourceVariableAccess],
    math: &crate::math_function_binding::ExpressionMathBindings<'_>,
) {
    use crate::tcl_expr_eval::{FoldPolicy, analyse_tcl_expr_with_integer_contents};
    let Some(normal) = &outcomes.normal else {
        return;
    };
    let Some(dialect) = state.baseline.dialect else {
        return;
    };
    let epoch = normal.source_variables.representation_epoch;
    let policy = FoldPolicy::for_retained_entry(context.registry, Some(dialect), &context.config);
    if policy.preparation_context().as_ref() != Some(preparation.witness.context()) {
        return;
    }
    if let Some(production) = preparation.witness.normal_numeric_result_production() {
        let mut shape = crate::native_numeric::SourceNativeNumericShape::from_expression(
            policy.invocation_dialect.unwrap_or(dialect),
            production,
        );
        if expression_has_integer_inputs(preparation, state, reads, context.registry) {
            shape = shape.with_integer_inputs();
        }
        super::source_representation::retain_expression_numeric_result(outcomes, shape);
    }
    let Some((environment, operands, integer_contents)) =
        crate::native_numeric::expression_operands_with_integer_contents(
            preparation.witness.tree(),
            |node| {
                let read = original_expression_read(preparation, node, reads)?;
                crate::native_numeric::source_read_operand(read, context.registry)
            },
            |node| {
                let read = original_expression_read(preparation, node, reads)?;
                crate::native_numeric::source_read_integer_contents(read, context.registry)
            },
            |node| {
                let crate::expr_ast::ExprNode::Call {
                    function, start, ..
                } = node
                else {
                    return false;
                };
                math.resolved_call(function, *start).is_some()
            },
        )
    else {
        return;
    };
    let Some(evaluation) = analyse_tcl_expr_with_integer_contents(
        preparation.witness.tree(),
        &environment,
        policy,
        &|function, start| {
            math.resolved_call(function, start)
                .map(|call| call.target())
        },
        &operands,
        &integer_contents,
    ) else {
        return;
    };
    // A successful selected evaluation is a completion witness only with the
    // independent original object/read and compiler obligations closed. This
    // removes the conservative getter-error alternatives, not opaque effects
    // or errors from callbacks. Result bytes remain a separate projection.
    if expression_evaluation_completes_normally(
        &evaluation,
        outcomes,
        preparation,
        state,
        reads,
        context.registry,
    ) {
        outcomes.abrupt.clear();
        outcomes.abrupt_values.clear();
        outcomes.abrupt_objects.clear();
        outcomes.retain_complete_normal_evaluation();
    }
    outcomes.normal_value =
        evaluated_numeric_source_value(&evaluation, preparation, policy, epoch).map(Arc::new);
}

fn evaluated_numeric_source_value(
    evaluation: &crate::tcl_expr_eval::FoldEvaluation,
    preparation: &SourceExpressionPreparation,
    policy: crate::tcl_expr_eval::FoldPolicy,
    epoch: Option<u64>,
) -> Option<super::native_result::EvaluatedSourceValue> {
    let dialect = policy.invocation_dialect?;
    // Contents on native normal completion are independent of the current
    // representation of a reused pool object. Retain native selected bytes
    // rather than formatting their mathematical interpretation.
    let text = match &evaluation.result_dependency {
        Some(crate::tcl_expr_eval::NativeExpressionResultDependency::StringResult { bytes }) => {
            Some(bytes.clone())
        }
        Some(crate::tcl_expr_eval::NativeExpressionResultDependency::SelectedOperand {
            existing_bytes,
            ..
        }) => existing_bytes.clone(),
        None => crate::tcl_expr_eval::format_tcl_value_with_policy(&evaluation.value, policy),
    }?;
    let numeric = epoch.filter(|_| {
        evaluation.native_value_effects_are_proved()
            && (preparation.witness.numeric_result_recipe()
                == tcl_registry::runtime_expr_validation::NativeExpressionResultRecipe::NormalizeOperand
                || preparation.witness.normal_numeric_result_production().is_some())
    }).map(|epoch| crate::native_numeric::FrozenSourceNumeric {
        object: Arc::new(crate::native_numeric::SourceNativeNumericObject {
            dialect,
            preparation: Arc::new(preparation.clone()),
            number: crate::native_numeric::SourceNativeNumber::from_value(&evaluation.value),
        }),
        epoch,
    });
    Some(super::native_result::EvaluatedSourceValue {
        text,
        representation: tcl_syntax::value::ValueRepresentation::Unknown,
        numeric,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ExpressionEvaluationIssuer {
    OriginalSource,
    EvaluatedSource,
}

/// Checked expression for its explicit source/evaluation purpose. The original
/// source projection grants no compiler entry, result object, read or totality.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct SourceConditionalExpressionEvaluation {
    issuer: ExpressionEvaluationIssuer,
    invocation: CommandAllocationSite,
    namespace_key: super::SourceNamespaceKey,
    frame: crate::var_resolve::VariableExecutionFrame,
    source: Arc<ExecutedScriptSource>,
    evaluation: tcl_registry::conditional_expression::ConditionalExpressionEvaluation,
    pool: tcl_registry::conditional_expression::ConditionalExpressionPoolState,
    lookup_closed: bool,
}

impl SourceConditionalExpressionEvaluation {
    pub(crate) const fn semantic_lookup_closed(&self) -> bool {
        self.lookup_closed
    }

    pub(crate) fn source(&self) -> &ExecutedScriptSource {
        &self.source
    }

    pub(crate) fn namespace_context(&self) -> &super::SourceNamespaceKey {
        &self.namespace_key
    }

    pub(crate) fn frame(&self) -> &crate::var_resolve::VariableExecutionFrame {
        &self.frame
    }

    pub(crate) fn tree(&self) -> &crate::expr_ast::ExprNode {
        self.evaluation.tree()
    }

    pub(crate) fn lexer_grammar(&self) -> tcl_dialect::LexerGrammar {
        self.evaluation.lexer_grammar()
    }

    pub(crate) fn parse_context(&self) -> &tcl_syntax::expr::parser::ExprParseContext {
        self.evaluation.context()
    }

    pub(crate) fn normal_result_representation(
        &self,
        variable: impl FnMut(&crate::expr_ast::ExprNode) -> Option<tcl_registry::TclType>,
        function: impl FnMut(
            &crate::expr_ast::ExprNode,
        ) -> Option<tcl_syntax::expr::mathfunc::MathResultClass>,
    ) -> Option<tcl_registry::TclType> {
        self.lookup_closed.then_some(())?;
        self.evaluation
            .normal_result_representation_with_functions(self.pool, variable, function)
    }
}

/// Preparation of an evaluated expression at its actual invocation entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceExpressionPreparation {
    /// Invocation selecting this expression after its arguments evaluated.
    pub invocation: CommandAllocationSite,
    /// Exact namespace incarnation at the original preparation entry.
    pub namespace_key: super::SourceNamespaceKey,
    /// Exact expression bytes and truthful authored or materialised origin.
    pub source: Arc<ExecutedScriptSource>,
    /// Native prepared tree, grammar and any actual fixed-table prerequisite.
    pub witness: Arc<PreparedExpressionWitness>,
    /// Exact native compiler visits required by a script-dependent preparation.
    pub script_compilation: Option<Arc<SourceExpressionScriptCompilation>>,
    /// Original operand pieces captured by the actual expression walker.
    /// This is independent of the derived expression's parser coordinates.
    #[doc(hidden)]
    pub executed_expression: Option<Arc<super::ExecutedExpressionSource>>,
}

/// Closed ordered script preparation, retaining its immutable compiler table.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceExpressionScriptCompilation {
    source: Arc<ExecutedScriptSource>,
    snapshot: super::NativeCompilationSnapshot,
    context: tcl_registry::native_compilation::NativeCompilationContext,
    visits: Vec<ExpressionScriptCompilerVisit>,
    dependencies: Vec<super::SourceNativeCompilationDependency>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ExpressionScriptCompilerVisit {
    span: tcl_lexer::Span,
    recipes: std::collections::BTreeMap<
        CommandAllocationSite,
        Vec<super::compiler_inventory::SourceCompilerInvocation>,
    >,
}

impl SourceExpressionPreparation {
    /// Match one unchanged written expression to its actual prepared bytes.
    /// Multi-operand concatenation and opaque mappings decline edit ownership.
    #[must_use]
    pub fn original_expression_mapping(&self) -> Option<(&Arc<super::SourceOriginId>, u32)> {
        if let Some(expression) = &self.executed_expression {
            if expression.origin != self.source.origin
                || expression.text.as_bytes() != self.source.text.bytes()
            {
                return None;
            }
            return expression.original_expression_mapping();
        }
        let super::ExecutedScriptMapping::Contiguous { base } = self.source.mapping else {
            return None;
        };
        Some((&self.source.origin, base))
    }

    /// Original variable occurrence retained before expression concatenation.
    /// Equal rendered offsets or a variable's base label cannot select a read.
    #[must_use]
    pub fn original_variable_source(
        &self,
        node: &crate::expr_ast::ExprNode,
    ) -> Option<(&Arc<super::SourceOriginId>, crate::ir::SourceSite)> {
        self.original_variable_source_with_config(
            node,
            tcl_lexer::LexerConfig::from_grammar(self.witness.context().lexer_grammar),
        )
    }

    /// Original occurrence geometry under the actual supplied whole-word
    /// configuration. This grants no read, frame or object proof itself.
    pub(crate) fn original_variable_source_with_config(
        &self,
        node: &crate::expr_ast::ExprNode,
        config: tcl_lexer::LexerConfig,
    ) -> Option<(&Arc<super::SourceOriginId>, crate::ir::SourceSite)> {
        if let Some(expression) = &self.executed_expression {
            if expression.origin != self.source.origin
                || expression.text.as_bytes() != self.source.text.bytes()
            {
                return None;
            }
            let original = expression.variable_source(node)?;
            return (original.0 == &self.invocation.source).then_some(original);
        }
        let super::ExecutedScriptMapping::Contiguous { base } = self.source.mapping else {
            return None;
        };
        if self.source.origin != self.invocation.source {
            return None;
        }
        Some((
            &self.source.origin,
            crate::ir::SourceSite::source(node.variable_source_span(base, config)?),
        ))
    }
    /// Script-dependent witnesses require the original ordered compiler receipts.
    #[must_use]
    pub fn has_closed_script_compilation(&self) -> bool {
        if self.witness.compiled_scripts().is_empty() {
            return self.script_compilation.is_none();
        }
        self.script_compilation.as_ref().is_some_and(|proof| {
            proof.source == self.source
                && self.witness.source().as_bytes() == self.source.text.bytes()
                && proof.visits.iter().map(|visit| visit.span).eq(self
                    .witness
                    .compiled_scripts()
                    .iter()
                    .copied())
                && proof.visits.iter().all(|visit| {
                    visit
                        .recipes
                        .keys()
                        .all(|site| site.source == self.source.origin)
                })
        })
    }

    /// Compiler-entry registration guards, independently of reached math calls.
    #[must_use]
    pub fn script_compilation_dependencies(&self) -> &[super::SourceNativeCompilationDependency] {
        self.script_compilation
            .as_ref()
            .map_or(&[], |proof| &proof.dependencies)
    }
}

impl super::SourceInvocationBinding {
    /// Original scoped normalization equivalence. Both command occurrences,
    /// their selected compiler alternatives, frame, world and observer envelopes
    /// remain closed; no actual entry or expression preparation is published.
    #[cfg(test)]
    pub(crate) fn nested_expression_normalisation(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &crate::ir::CommandTokens,
        parser: &tcl_syntax::expr::parser::ExprParseContext,
        expected: Option<&crate::expr_ast::ExprNode>,
    ) -> bool {
        self.nested_expression_normalisation_source(registry, tokens, parser, expected)
            .is_some()
    }

    /// Same scoped normalization proof, retaining the selected inner expression
    /// and its original command-substitution source. Strings are projections of
    /// those actual owners, never command identities parsed from report text.
    pub(crate) fn nested_expression_normalisation_source(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &crate::ir::CommandTokens,
        parser: &tcl_syntax::expr::parser::ExprParseContext,
        expected: Option<&crate::expr_ast::ExprNode>,
    ) -> Option<(String, String)> {
        let outer = self.normalisation_expression(registry, tokens)?;
        if outer.evaluation.context() != parser || expected.is_some_and(|tree| tree != outer.tree())
        {
            return None;
        }
        let crate::expr_ast::ExprNode::Command { start, end, .. } = outer.tree() else {
            return None;
        };
        let begin = outer.source.base().checked_add(*start)?;
        let command_end = outer.source.base().checked_add(*end)?;
        let surface = tcl_registry::model::DocumentCommandSurface::new(registry, None);
        let calls = crate::word_subst::lifted_calls_with_surface(
            Some(tokens),
            self.original_argument_lexer_config(tokens)?,
            &surface,
        );
        let mut inner = calls.iter().filter(|call| {
            call.tokens
                .as_ref()
                .and_then(|tokens| tokens.source_binding.as_ref())
                .and_then(super::SourceInvocationBinding::invocation_site)
                .is_some_and(|site| {
                    site.source == outer.invocation.source
                        && site.offset > begin
                        && site.offset < command_end
                })
        });
        let tokens = inner.next()?.tokens.as_ref()?;
        if inner.next().is_some() {
            return None;
        }
        let binding = tokens.source_binding.as_ref()?;
        let inner = binding.normalisation_expression(registry, tokens)?;
        (outer.frame == inner.frame
            && outer.namespace_key == inner.namespace_key
            && self.runtime_reachability == binding.runtime_reachability
            && outer
                .evaluation
                .numeric_reentry_is_idempotent(&inner.evaluation))
        .then_some(())?;
        Some((
            inner.source.try_text().ok()?.to_owned(),
            outer
                .source
                .try_text()
                .ok()?
                .get(usize::try_from(*start).ok()?..usize::try_from((*end).checked_add(1)?).ok()?)?
                .to_owned(),
        ))
    }

    /// Reborrow one selected operation's original written operand inside an
    /// exact original expression. This source projection supplies no reached
    /// implementation, preparation, effect absence or rewrite permission.
    pub(crate) fn original_expression_child_word(
        &self,
        registry: &tcl_registry::CommandRegistry,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        tokens: &crate::ir::CommandTokens,
        expression: (&crate::expr_ast::ExprNode, &str, u32),
        child: (
            &crate::expr_ast::ExprNode,
            tcl_registry::SemanticOperationId,
        ),
    ) -> Option<tcl_lexer::NativeWord> {
        if let Some(input) = metadata.source_analysis_input() {
            self.original_argument_metadata_for_input(tokens, input, registry)?;
        } else if !metadata.is_standalone() {
            return None;
        }
        let (expected, source, base) = expression;
        let (command, operation) = child;
        let crate::expr_ast::ExprNode::Command { start, end, .. } = command else {
            return None;
        };
        let belongs = match expected {
            crate::expr_ast::ExprNode::Command { .. } => expected == command,
            crate::expr_ast::ExprNode::Binary { left, right, .. } => {
                left.as_ref() == command || right.as_ref() == command
            }
            _ => false,
        };
        if !belongs {
            return None;
        }
        let mut expression = None;
        for written in 0..tokens.words().len() {
            let Some(candidate) =
                self.conditional_expression_evaluation_for_original_word(registry, tokens, written)
            else {
                continue;
            };
            if candidate.source.base() == base
                && candidate.source.try_text().ok()? == source
                && candidate.tree() == expected
            {
                if expression
                    .as_ref()
                    .is_some_and(|previous| previous != &candidate)
                {
                    return None;
                }
                expression = Some(candidate);
            }
        }
        let expression = expression?;
        let span = tcl_lexer::Span::new(
            base.checked_add(*start)?,
            base.checked_add(end.checked_add(1)?)?,
        );
        let config = self.original_argument_lexer_config(tokens)?;
        let surface = tcl_registry::model::DocumentCommandSurface::new(registry, None);
        let calls = crate::word_subst::lifted_calls_with_surface(Some(tokens), config, &surface);
        let mut calls = calls.iter().filter(|call| call.span == span);
        let child = calls.next()?.tokens.as_ref()?;
        if calls.next().is_some() {
            return None;
        }
        let binding = child.source_binding.as_ref()?;
        let site = binding.invocation_site()?;
        if site.source != expression.source.origin {
            return None;
        }
        let invocation =
            crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
                registry,
                Some(metadata),
                child,
            )?;
        if invocation.facts.operation != operation {
            return None;
        }
        let argument = invocation.with_metadata_schema(
            registry,
            metadata,
            tcl_dialect::model::InvocationRealm::RuleLoader,
            |selected| {
                let index = selected.semantics.argument_offset;
                (selected.words.arguments().exact_argv_len() == index.checked_add(1))
                    .then_some(index)
            },
        )?;
        let written = invocation.effective.written_argument(argument)?;
        let original = site
            .source
            .original_lexical_command(child.words(), site.offset, config)?;
        let word = original.native_words().get(written.checked_add(1)?)?;
        (!word.group().expand).then(|| word.clone())
    }

    fn normalisation_expression(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<SourceConditionalExpressionEvaluation> {
        if tokens.source_binding.as_ref() != Some(self) {
            return None;
        }
        let target = if self.runtime_reachability == super::SourceRuntimeReachability::Conditional {
            self.scoped_unobserved_target(tokens)?
        } else {
            self.unobserved_native_dispatch().then_some(())?;
            self.proved_execution_target()?
        };
        if !target.registry_backed || target.kind != super::BindingKind::Builtin {
            return None;
        }
        let expression = self.conditional_expression_evaluation(registry, tokens)?;
        expression.semantic_lookup_closed().then_some(expression)
    }

    pub(super) fn conditional_expression_evaluation_for_original_word(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &crate::ir::CommandTokens,
        written: usize,
    ) -> Option<SourceConditionalExpressionEvaluation> {
        let advice = crate::registry_invocation::original_expression_operand_advice_for_word(
            registry, tokens, written,
        )?;
        let site = self.invocation_site()?;
        Some(SourceConditionalExpressionEvaluation {
            issuer: ExpressionEvaluationIssuer::OriginalSource,
            invocation: site.clone(),
            namespace_key: advice.namespace_key,
            frame: advice.frame,
            source: Arc::new(ExecutedScriptSource::contiguous(
                Arc::clone(&site.source),
                &advice.expression_text,
                advice.expression_base,
            )?),
            evaluation: advice.evaluation,
            pool: advice.pool,
            lookup_closed: advice.lookup_closed,
        })
    }

    /// Unanimous original conditional grammar and current authored pool state.
    /// A complete written operand mapping and declaration/handler selection are
    /// checked independently of actual expression preparation.
    pub(crate) fn conditional_expression_evaluation(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<SourceConditionalExpressionEvaluation> {
        let advice =
            crate::registry_invocation::original_expression_operand_advice(registry, tokens)?;
        let site = self.invocation_site()?;
        let constructed = SourceConditionalExpressionEvaluation {
            issuer: ExpressionEvaluationIssuer::OriginalSource,
            invocation: site.clone(),
            namespace_key: advice.namespace_key.clone(),
            frame: advice.frame.clone(),
            source: Arc::new(ExecutedScriptSource::contiguous(
                Arc::clone(&site.source),
                &advice.expression_text,
                advice.expression_base,
            )?),
            evaluation: advice.evaluation.clone(),
            pool: advice.pool,
            lookup_closed: advice.lookup_closed,
        };
        let Some(observations) = self.conditional_expression_evaluations.as_deref() else {
            return Some(constructed);
        };
        // Runtime evaluation can concatenate or materialise an expression
        // source. That issuer owns different bytes and a later pool point;
        // it cannot certify or contradict the original source projection.
        let mut original = observations
            .iter()
            .filter(|observation| observation.issuer == ExpressionEvaluationIssuer::OriginalSource);
        let Some(first) = original.next() else {
            return Some(constructed);
        };
        let valid = |observation: &SourceConditionalExpressionEvaluation| {
            &observation.invocation == site
                && observation.frame == advice.frame
                && observation.namespace_key == advice.namespace_key
                && observation.source.origin == site.source
                && observation.source.base() == advice.expression_base
                && matches!(
                    observation.source.mapping,
                    super::ExecutedScriptMapping::Contiguous { .. }
                )
                && observation.evaluation == advice.evaluation
                && observation.pool == advice.pool
        };
        (valid(first) && original.all(|observation| valid(observation) && observation == first))
            .then_some(constructed)
    }
    /// Complete evaluation of the original arguments at this exact source
    /// invocation, conditional on its retained entry. This is independent of
    /// successful handler dispatch, SSA definitions and a concrete result.
    #[must_use]
    pub fn original_arguments_complete_normally(&self, tokens: &crate::ir::CommandTokens) -> bool {
        if self.runtime_reachability == super::SourceRuntimeReachability::Conditional
            || !self.original_argument_completion.is_normal()
            || tokens.synthetic.is_some()
            || self.entered_execution_observer != super::SourceObserverExecution::Unobserved
        {
            return false;
        }
        let Some(site) = self.dispatch_site.as_ref() else {
            return false;
        };
        let Some(original) =
            crate::registry_invocation::native_compiler_replay_source(tokens, site)
        else {
            return false;
        };
        let Some(dialect) = self.variable_context.invocation_dialect else {
            return false;
        };
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        let image = tcl_lexer::SourceImage::from_bytes(
            original.as_bytes(),
            site.source.source_image().channel(),
        );
        let Some(segments) = crate::segmenter::segment_commands_image_with_offset_and_config(
            &image,
            site.offset,
            config,
        ) else {
            return false;
        };
        let [segment] = segments.as_slice() else {
            return false;
        };
        let original_tokens = crate::ir::CommandTokens::from_segmented(
            &site.source.source_image().source_map(),
            config,
            segment,
        );
        original_tokens.words() == tokens.words() && original_tokens.argv_texts == tokens.argv_texts
    }

    /// Original preparation belonging to this exact evaluated invocation.
    /// Conflicting source, compiler or preparation observations decline. This
    /// supplies a conditional expression result, never completion or opcode authority.
    #[must_use]
    pub fn expression_preparation<'a>(
        &self,
        preparations: &'a [SourceExpressionPreparation],
    ) -> Option<&'a SourceExpressionPreparation> {
        if self.runtime_reachability == super::SourceRuntimeReachability::Conditional {
            return None;
        }
        let site = self.dispatch_site.as_ref()?;
        let mut candidates = preparations
            .iter()
            .filter(|proof| &proof.invocation == site);
        let first = candidates.next()?;
        (first.has_closed_script_compilation()
            && first.witness.source().as_bytes() == first.source.text.bytes()
            && candidates.all(|proof| proof == first))
        .then_some(first)
    }
}

impl SourceCommandBindings {
    /// Closed reached preparation proofs owned by this exact script's invocations.
    #[must_use]
    pub fn expression_preparations_for_script(
        &self,
        script: &ExecutedScriptSource,
    ) -> Vec<SourceExpressionPreparation> {
        #[cfg(any(test, debug_assertions))]
        let _timing =
            super::root_owner_work::Timing::at(super::root_owner_work::Query::BodyExpression);
        self.expression_preparations
            .iter()
            .filter(|(site, _)| {
                site.source == script.origin
                    && site.offset >= script.base()
                    && u64::from(site.offset) < u64::from(script.base()) + script.text.len() as u64
            })
            .filter_map(|(_, proofs)| proofs.as_ref())
            .flatten()
            .cloned()
            .collect()
    }

    fn record_evaluated_expression_topology(
        &mut self,
        invocation: &CommandAllocationSite,
        source: &ExecutedScriptSource,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) {
        // Preparation and reached operands may shimmer shared native values,
        // including expression operands of control handlers. Retain their
        // bytes/cell identities while withdrawing unproved representation.
        if let (Some(dialect), Ok(text)) = (state.baseline.dialect, source.try_text()) {
            let mut syntax = dialect.expression_parse_context_with(
                state.baseline.expression_parser,
                context.registry.profile(),
            );
            syntax.lexer_grammar = context.config.grammar_over(syntax.lexer_grammar);
            if let Some(evaluation) =
                tcl_registry::conditional_expression::ConditionalExpressionEvaluation::prepare(
                    text, &syntax,
                )
            {
                use tcl_registry::conditional_expression::ConditionalExpressionPoolState as Pool;
                let pool = if !state.opaque_domain
                    && !state.source_variables.dynamic_traces
                    && state.ordinary_literal_pool.as_ref().is_some_and(
                        super::literal_object_pool::SourceOrdinaryLiteralPool::initial_numeric_representations,
                    )
                { Pool::InitialAuthoredPool } else { Pool::Unknown };
                let observation = SourceConditionalExpressionEvaluation {
                    issuer: ExpressionEvaluationIssuer::EvaluatedSource,
                    invocation: invocation.clone(),
                    namespace_key: context.namespace_identity(),
                    frame: state.variable_frame.clone(),
                    source: Arc::new(source.clone()),
                    evaluation,
                    pool,
                    // Handler-walk topology does not own original lookup completeness.
                    lookup_closed: false,
                };
                let observations = self
                    .conditional_expression_evaluations
                    .entry(invocation.clone())
                    .or_default();
                if !observations.contains(&observation) {
                    observations.push(observation);
                }
            }
        }
    }

    fn walk_prepared_expression_witness(
        &mut self,
        invocation: CommandAllocationSite,
        proof: &SourceExpressionPreparation,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        invalidate_preparation_representations(proof.witness.tree(), state, context);
        // A hosted F5 rule's handler bodies are only ever walked as advice;
        // their prepared conditions are retained for the advisory solve.
        if self.declaration_preview_depth == 0
            || super::logical_definition::hosted_entry(state, context).is_some()
        {
            let entry = self
                .expression_preparations
                .entry(invocation)
                .or_insert_with(|| Some(Vec::new()));
            if let Some(proofs) = entry
                && !proofs.contains(proof)
            {
                proofs.push(proof.clone());
            }
        }
        let rhs_read =
            super::read_store_schedule::CapturedExpressionRead::capture(proof, state, context);
        let numeric_cache_entry = Arc::clone(&state.source_variables);
        let root_coercion = root_result_coercion(proof.witness.tree(), state, context);
        let capture = self.begin_expression_read_capture(proof, &state.variable_frame);
        let mut outcomes =
            self.walk_expression(proof.witness.tree(), proof.source.base(), state, context);
        let mut reads = self.finish_expression_read_capture(capture, proof);
        if outcomes.normal.is_none() {
            reads.clear();
        }
        discard_normal_operand_results(&mut outcomes);
        if let Some(normal) = &mut outcomes.normal
            && let Some(coercion) = root_coercion
        {
            coercion.apply(normal);
        }
        super::numeric_operand_cache::finish_expression(
            &mut outcomes,
            proof,
            state,
            &numeric_cache_entry,
            &reads,
            context,
        );
        let math_calls = self.implicit_math_invocations_for_script(&proof.source);
        let math = crate::math_function_binding::ExpressionMathBindings::for_origin(
            &math_calls,
            Some(&proof.source),
            Some(proof.source.base()),
        );
        native_numeric_result(&mut outcomes, proof, state, context, &reads, &math);
        if integer_expression_completes_normally(proof, state, &reads, context) {
            outcomes.retain_complete_normal_evaluation();
        }
        outcomes.normal_rhs_read = self.completed_expression_read(
            rhs_read,
            &outcomes,
            proof.invocation.offset,
            context.registry,
        );
        outcomes.publish(state);
        outcomes
    }

    pub(super) fn walk_prepared_expression(
        &mut self,
        invocation: CommandAllocationSite,
        source: ExecutedScriptSource,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::completion::CompletionCode;
        use tcl_registry::completion_route::InvocationCompletionRoute;
        self.record_evaluated_expression_topology(&invocation, &source, state, context);
        let (preparation, script_compilation) = prepare_reached_expression(&source, state, context);
        match preparation {
            ExpressionPreparationProof::Prepared(witness) => {
                let proof = SourceExpressionPreparation {
                    invocation: invocation.clone(),
                    namespace_key: context.namespace_identity(),
                    source: Arc::new(source),
                    witness: Arc::from(witness),
                    script_compilation,
                    executed_expression: context.expression_source.cloned().map(Arc::new),
                };
                self.walk_prepared_expression_witness(invocation, &proof, state, context)
            }
            ExpressionPreparationProof::Rejected(_) => {
                Arc::make_mut(&mut state.source_variables).invalidate_shared_representations();
                if self.declaration_preview_depth == 0 {
                    self.expression_preparations.insert(invocation, None);
                }
                super::compiled_preflight::materialise_error_storage(state, context.registry);
                SourceOutcomes::invocation(
                    state,
                    InvocationCompletionRoute::Tcl(CompletionCode::Error),
                )
            }
            ExpressionPreparationProof::Unknown => {
                Arc::make_mut(&mut state.source_variables).invalidate_shared_representations();
                if self.declaration_preview_depth == 0 {
                    self.expression_preparations.insert(invocation, None);
                }
                if state
                    .baseline
                    .dialect
                    .is_none_or(|dialect| dialect.family() == Some(tcl_dialect::model::Family::Jim))
                {
                    // Jim's eager preparation can regroup calls; its ordinary
                    // syntax tree does not prove the executed operand topology.
                    return super::opaque_source_invocation(state);
                }
                // Absence of a preparation witness preserves a possible entry
                // rejection. It does not itself mutate the successful handler's
                // world; reached effects still come from the source evaluator.
                let mut rejected = super::boxed_source_branch(state);
                super::compiled_preflight::materialise_error_storage(
                    &mut rejected,
                    context.registry,
                );
                let Ok(source_text) = source.try_text() else {
                    return super::opaque_source_invocation(state);
                };
                let expression = if let Some(dialect) = state.baseline.dialect {
                    let mut syntax = dialect.expression_parse_context_with(
                        state.baseline.expression_parser,
                        context.registry.profile(),
                    );
                    syntax.lexer_grammar = context.config.grammar_over(syntax.lexer_grammar);
                    crate::expr_parser::parse_expr_with_syntax_context(source_text, &syntax)
                } else {
                    super::parse_source_expression(source_text, state, context.registry)
                };
                let mut outcomes = self.walk_expression(&expression, source.base(), state, context);
                discard_normal_operand_results(&mut outcomes);
                outcomes.join(&SourceOutcomes::invocation(
                    &rejected,
                    InvocationCompletionRoute::Tcl(CompletionCode::Error),
                ));
                outcomes.publish(state);
                outcomes
            }
        }
    }
}

// Keep the full parser/table descriptors out of recursive evaluation frames.
#[inline(never)]
fn prepare_reached_expression(
    source: &ExecutedScriptSource,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> (
    ExpressionPreparationProof,
    Option<Arc<SourceExpressionScriptCompilation>>,
) {
    let Some(dialect) = state.baseline.dialect else {
        return (ExpressionPreparationProof::Unknown, None);
    };
    let Ok(source_text) = source.try_text() else {
        return (ExpressionPreparationProof::Unknown, None);
    };
    let mut parser = dialect.expression_parse_context_with(
        state.baseline.expression_parser,
        context.registry.profile(),
    );
    parser.lexer_grammar = context.config.grammar_over(parser.lexer_grammar);
    let table = (!state.opaque_domain)
        .then(|| state.baseline.native_entry.as_ref())
        .flatten()
        .and_then(|entry| {
            entry.math_functions.clone().map(|table| {
                tcl_runtime_api::native_compilation::NativeMathFunctionPrerequisite {
                    interpreter: entry.interpreter,
                    table,
                }
            })
        });
    let mut snapshot = None;
    let mut closure = None;
    let preparation = prepare_expression_witness_with_script_compiler(
        source_text,
        &parser,
        table.as_ref(),
        |span| {
            let snapshot = snapshot.get_or_insert_with(|| {
                context
                    .compilation_snapshot
                    .cloned()
                    .unwrap_or_else(|| state.native_compilation_snapshot_in_realm(context.realm()))
            });
            let closure = closure.get_or_insert_with(|| SourceExpressionScriptCompilation {
                source: Arc::new(source.clone()),
                snapshot: snapshot.clone(),
                context: context.compilation(),
                visits: Vec::new(),
                dependencies: Vec::new(),
            });
            compile_expression_script(source, span, state, context, snapshot, closure)
        },
    );
    let closure = closure
        .filter(|proof| {
            !proof.visits.is_empty()
                && matches!(preparation, ExpressionPreparationProof::Prepared(_))
        })
        .map(Arc::new);
    (preparation, closure)
}

fn compile_expression_script(
    source: &ExecutedScriptSource,
    span: tcl_lexer::Span,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    snapshot: &mut super::NativeCompilationSnapshot,
    closure: &mut SourceExpressionScriptCompilation,
) -> NativeExpressionScriptPreparation {
    use tcl_registry::native_compilation::{NativeCompilationGuard, NativeCompilationMode};
    let Some(script) = source
        .try_text()
        .ok()
        .and_then(|text| text.get(span.as_range()))
        .and_then(|spelling| {
            spelling
                .strip_prefix('[')
                .and_then(|text| text.strip_suffix(']'))
        })
    else {
        return NativeExpressionScriptPreparation::Unknown;
    };
    let Some(base) = source
        .base()
        .checked_add(span.start())
        .and_then(|base| base.checked_add(1))
    else {
        return NativeExpressionScriptPreparation::Unknown;
    };
    let mut compiler_state = state.clone();
    compiler_state.current_source_origin = Some(Arc::clone(&source.origin));
    let mut compilation = context.compilation();
    compilation.mode = NativeCompilationMode::BytecodeObject;
    let assessment = super::compiled_preflight::preflight(
        script,
        base,
        &compiler_state,
        SourceExecutionContext {
            control: (context).control.with_compilation(compilation),
            compilation_snapshot: None,
            ..context
        },
        snapshot,
    );
    if let Some(failure) = assessment.failure {
        return NativeExpressionScriptPreparation::Rejected(Box::new(failure.failure));
    }
    if assessment.possible_error {
        return NativeExpressionScriptPreparation::Unknown;
    }
    for recipes in assessment.compiler_invocations.values() {
        for recipe in recipes {
            let Some(head) = &recipe.head else {
                continue;
            };
            let targets = recipe
                .table
                .state
                .native_compiler_targets(head, &recipe.namespace_key);
            if targets.unknown || targets.may_be_generic || targets.targets.len() != 1 {
                return NativeExpressionScriptPreparation::Unknown;
            }
            let target = targets
                .targets
                .into_iter()
                .next()
                .expect("single compiler target");
            let dependency = super::SourceNativeCompilationDependency {
                compiler_prerequisite: recipe
                    .table
                    .state
                    .runtime_command_compiler_prerequisite(
                        head,
                        &recipe.namespace_key,
                        tcl_runtime_api::CommandBindingGuard::ChunkEntry,
                    )
                    .map(Arc::new),
                target,
                namespace: recipe.namespace.clone(),
                namespace_key: recipe.namespace_key.clone(),
                head: head.clone(),
                guard: NativeCompilationGuard::ChunkEntry,
            };
            if !closure.dependencies.contains(&dependency) {
                closure.dependencies.push(dependency);
            }
            if let Some(admitted) = &recipe.admitted {
                let dependencies = match admitted.as_ref() {
                    super::compiled_invocation::SourceNativeCompilerAdmission::Inline(proof) => {
                        &proof.lookup_dependencies
                    }
                    super::compiled_invocation::SourceNativeCompilerAdmission::Named(proof) => {
                        &proof.dependencies
                    }
                };
                for dependency in dependencies {
                    if !closure.dependencies.contains(dependency) {
                        closure.dependencies.push(dependency.clone());
                    }
                }
            }
        }
    }
    closure.visits.push(ExpressionScriptCompilerVisit {
        span,
        recipes: assessment.compiler_invocations,
    });
    NativeExpressionScriptPreparation::Compiled
}

#[cfg(test)]
mod tests;
