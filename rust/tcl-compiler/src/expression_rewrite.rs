// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Execution equivalence for proposed expression rewrites, before substitution.

use crate::expr_ast::ExprNode;
use crate::math_function_binding::ExpressionMathBindings;
use crate::tcl_expr_eval::{
    Env, FoldPolicy, NativeOperandProofs, analyse_tcl_expr_with_resolved_math_bindings,
};

/// Why mathematical simplification cannot establish executable equivalence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressionRewriteDecline {
    /// Original source preparation or selected grammar is not proved.
    PreparationUnknown,
    /// The original or proposed normal result is unresolved.
    ValueUnknown,
    /// A reached retained object conversion must remain executed.
    RetainedObjectCoercion,
    /// Native selected-object/string result identity or spelling must remain.
    NativeResultDependency,
    /// A produced numeric result would become a selected or literal object.
    NativeResultAllocation,
    /// The normal values differ under the selected native semantics.
    ValueChanged,
}

/// Compare native result protocols independently of their mathematical values.
/// `None` represents a contents-only replacement: a pooled literal in place of
/// the original result object. That is admitted where C pools the result
/// itself, or where the selected operator always yields an integer, whose
/// canonical spelling leaves only object identity and representation changed.
/// Keeping an expression requires the same selected result instruction family;
/// operand, error and value equivalence remain separate obligations.
pub fn expression_result_protocol_equivalence(
    original: &tcl_registry::runtime_expr_validation::PreparedExpressionWitness,
    replacement: Option<&tcl_registry::runtime_expr_validation::PreparedExpressionWitness>,
) -> Result<(), ExpressionRewriteDecline> {
    use tcl_registry::runtime_expr_validation::NativeExpressionResultRecipe as Recipe;
    let recipe = original.numeric_result_recipe();
    if recipe == Recipe::Unknown {
        return Err(ExpressionRewriteDecline::PreparationUnknown);
    }
    let equivalent = match replacement {
        Some(replacement) => replacement.numeric_result_recipe() == recipe,
        None => match recipe {
            Recipe::PooledValue => true,
            Recipe::NumericOperation => original
                .normal_numeric_result_production()
                .is_some_and(|production| production.result_type() == tcl_registry::TclType::Int),
            Recipe::NormalizeOperand | Recipe::Unknown => false,
        },
    };
    if equivalent {
        Ok(())
    } else {
        Err(ExpressionRewriteDecline::NativeResultAllocation)
    }
}

/// A contents-only replacement for an evaluated normal result. Beyond
/// [`expression_result_protocol_equivalence`], an arithmetic operator whose
/// evaluated result is an integer is admitted: C then produces an integer
/// object with the same canonical spelling. A double keeps its operation,
/// since its spelling follows `tcl_precision` when its string is generated;
/// comparison and logical results, operand normalisation and unknown
/// recipes keep theirs too.
pub fn contents_only_result_equivalence(
    original: &tcl_registry::runtime_expr_validation::PreparedExpressionWitness,
    value: &crate::tcl_expr_eval::TclValue,
) -> Result<(), ExpressionRewriteDecline> {
    use crate::tcl_expr_eval::TclValue;
    use tcl_registry::runtime_expr_validation::NativeExpressionResultRecipe as Recipe;
    match expression_result_protocol_equivalence(original, None) {
        Err(ExpressionRewriteDecline::NativeResultAllocation)
            if original.numeric_result_recipe() == Recipe::NumericOperation
                && original.normal_numeric_result_production().is_some()
                && matches!(value, TclValue::Int(_) | TclValue::Big(_)) =>
        {
            Ok(())
        }
        result => result,
    }
}

/// Prove equivalence without treating newly substituted literals as fresh
/// original execution operands. `original` must be the retained original AST.
/// Mathematical type/value facts do not establish actual operand-object proof.
/// Missing partial-effect equivalence is a typed decline, not an erasure licence.
pub fn expression_rewrite_equivalence(
    original: &ExprNode,
    proposed: &str,
    environment: &Env,
    policy: FoldPolicy,
    bindings: Option<ExpressionMathBindings<'_>>,
    operands: Option<&NativeOperandProofs>,
) -> Result<(), ExpressionRewriteDecline> {
    use ExpressionRewriteDecline as Decline;
    use tcl_registry::runtime_expr_validation::{
        ExpressionPreparationProof, prepare_expression_witness,
    };
    let context = policy
        .preparation_context()
        .ok_or(Decline::PreparationUnknown)?;
    let preparation = bindings.and_then(|bindings| bindings.preparation_for_context(&context));
    if bindings.is_some_and(|bindings| bindings.is_positioned()) && preparation.is_none() {
        return Err(Decline::PreparationUnknown);
    }
    let original = preparation.map_or(original, |preparation| preparation.witness.tree());
    let before = analyse_tcl_expr_with_resolved_math_bindings(
        original,
        environment,
        policy,
        &|function, start| {
            let call = bindings?.resolved_call(function, start)?;
            crate::math_function_binding::native_fold_dependency(call.invocation)?;
            Some(call.target())
        },
        operands,
    )
    .ok_or(Decline::ValueUnknown)?;
    if !before.coercions.is_empty() {
        return Err(Decline::RetainedObjectCoercion);
    }
    if before.result_dependency.is_some() {
        return Err(Decline::NativeResultDependency);
    }
    let ExpressionPreparationProof::Prepared(proposed) = prepare_expression_witness(
        proposed,
        &context,
        preparation.and_then(|preparation| preparation.witness.fixed_functions()),
    ) else {
        return Err(Decline::PreparationUnknown);
    };
    if let Some(original) = preparation {
        expression_result_protocol_equivalence(&original.witness, Some(&proposed))?;
    }
    if contains_retained_operand(original)
        && numeric_result_operation(original)
        && !numeric_result_operation(proposed.tree())
    {
        // An already numeric input closes its conversion obligation, but the
        // arithmetic result still has its own object. Selecting the input or
        // a pooled literal can introduce sharing observed by later coercions.
        return Err(Decline::NativeResultAllocation);
    }
    // Rendered proposed text has no original implicit-call site mapping. It
    // cannot inherit the original tree's function proof by coincident offsets.
    let after = analyse_tcl_expr_with_resolved_math_bindings(
        proposed.tree(),
        environment,
        policy,
        &|_, _| None,
        operands,
    )
    .ok_or(Decline::ValueUnknown)?;
    if !after.coercions.is_empty() {
        return Err(Decline::RetainedObjectCoercion);
    }
    if after.result_dependency.is_some() {
        return Err(Decline::NativeResultDependency);
    }
    (crate::native_numeric::SourceNativeNumber::from_value(&before.value)
        == crate::native_numeric::SourceNativeNumber::from_value(&after.value))
    .then_some(())
    .ok_or(Decline::ValueChanged)
}

/// A closed comparison rewrite preserves both original operands and one
/// successful already-numeric read. A fixed nonnumeric literal cannot compare
/// equal to any native numeric value, including infinities or NaN. Both forms
/// preserve current numeric representations under the selected fallback and
/// retain the Boolean result protocol;
/// no representative numeric value or executable constant is manufactured.
pub(crate) fn numeric_string_comparison_equivalence(
    proposed: &str,
    policy: FoldPolicy,
    bindings: ExpressionMathBindings<'_>,
    registry: &tcl_registry::CommandRegistry,
) -> Result<(), ExpressionRewriteDecline> {
    use crate::expr_ast::BinOp;
    use tcl_registry::runtime_expr_validation::{
        ExpressionPreparationProof, prepare_expression_witness,
    };
    let context = policy
        .preparation_context()
        .ok_or(ExpressionRewriteDecline::PreparationUnknown)?;
    let original = bindings
        .preparation_for_context(&context)
        .ok_or(ExpressionRewriteDecline::PreparationUnknown)?;
    let ExpressionPreparationProof::Prepared(replacement) =
        prepare_expression_witness(proposed, &context, None)
    else {
        return Err(ExpressionRewriteDecline::PreparationUnknown);
    };
    let (
        ExprNode::Binary { op, left, right },
        ExprNode::Binary {
            op: new_op,
            left: new_left,
            right: new_right,
        },
    ) = (original.witness.tree(), replacement.tree())
    else {
        return Err(ExpressionRewriteDecline::ValueUnknown);
    };
    if !matches!(
        (op, new_op),
        (BinOp::Eq, BinOp::StrEq) | (BinOp::Ne, BinOp::StrNe)
    ) || !same_comparison_operand(left, new_left)
        || !same_comparison_operand(right, new_right)
    {
        return Err(ExpressionRewriteDecline::ValueUnknown);
    }
    if original.witness.equality_fallback_protocol()
        != Some(tcl_registry::runtime_expr_validation::NativeEqualityFallbackProtocol::PreservesNumericRepresentation)
    {
        return Err(ExpressionRewriteDecline::RetainedObjectCoercion);
    }
    let closed = [
        (left.as_ref(), right.as_ref()),
        (right.as_ref(), left.as_ref()),
    ]
    .into_iter()
    .any(|(variable, literal)| {
        comparison_literal_cannot_be_numeric(literal, policy, &context)
            && bindings.source_read_already_numeric(variable, &context, policy, registry)
    });
    if !closed {
        return Err(ExpressionRewriteDecline::RetainedObjectCoercion);
    }
    expression_result_protocol_equivalence(&original.witness, Some(&replacement))
}

fn same_comparison_operand(original: &ExprNode, replacement: &ExprNode) -> bool {
    match (original, replacement) {
        (
            ExprNode::Var { text: original, .. },
            ExprNode::Var {
                text: replacement, ..
            },
        )
        | (
            ExprNode::String { text: original, .. },
            ExprNode::String {
                text: replacement, ..
            },
        ) => original == replacement,
        _ => false,
    }
}

fn comparison_literal_cannot_be_numeric(
    node: &ExprNode,
    policy: FoldPolicy,
    context: &tcl_syntax::expr::parser::ExprParseContext,
) -> bool {
    let ExprNode::String { text, .. } = node else {
        return false;
    };
    let Some(value) = tcl_syntax::expr::fixed_string_operand(text) else {
        return false;
    };
    let Some(numbers) = policy.numbers else {
        return false;
    };
    // Reject prefixes and incomplete numeric-looking words too. Unknown values
    // and backslash/substitution-bearing strings never enter this closed case.
    value.is_ascii()
        && !value.is_empty()
        && !value.starts_with(['+', '-'])
        && !value.bytes().any(|byte| byte.is_ascii_whitespace())
        && tcl_syntax::boolean::parse_boolean_word(value).is_none()
        && tcl_syntax::number::parse_whole_with(
            value,
            tcl_syntax::number::ParseFlags::for_syntax(numbers),
        )
        .is_none()
        && tcl_dialect::scan_expr_number(value.as_bytes(), 0, numbers, context.expr_grammar_base)
            .is_none()
}

fn contains_retained_operand(tree: &ExprNode) -> bool {
    let mut pending = vec![tree];
    while let Some(node) = pending.pop() {
        match node {
            ExprNode::Var { .. } => return true,
            ExprNode::Binary { left, right, .. } => pending.extend([left.as_ref(), right.as_ref()]),
            ExprNode::Unary { operand, .. } => pending.push(operand),
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => pending.extend([
                condition.as_ref(),
                true_branch.as_ref(),
                false_branch.as_ref(),
            ]),
            _ => {}
        }
    }
    false
}

fn numeric_result_operation(tree: &ExprNode) -> bool {
    use crate::expr_ast::{BinOp, UnaryOp};
    matches!(
        tree,
        ExprNode::Binary {
            op: BinOp::Add
                | BinOp::Sub
                | BinOp::Mul
                | BinOp::Div
                | BinOp::Mod
                | BinOp::Pow
                | BinOp::LShift
                | BinOp::RShift
                | BinOp::BitAnd
                | BinOp::BitOr
                | BinOp::BitXor,
            ..
        } | ExprNode::Unary {
            op: UnaryOp::Neg | UnaryOp::BitNot,
            ..
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tcl_expr_eval::EnvValue;

    #[test]
    fn a_pooled_result_cannot_be_replaced_by_numeric_normalisation() {
        let source = "set result [expr {2+2}]";
        let owner = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = owner.commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            source,
            registry,
            false,
            registry.profile().unwrap(),
        );
        let statement = &unit.ir_module.top_level.statements[0];
        let crate::ir::Statement::AssignExpr { expr, span, .. } = statement else {
            panic!("original retained expression");
        };
        let bindings =
            ExpressionMathBindings::for_module_statement(&unit.ir_module, *span).unwrap();
        assert_eq!(
            expression_rewrite_equivalence(
                expr,
                "4",
                &Env::new(),
                FoldPolicy::from_registry(registry),
                Some(bindings),
                None,
            ),
            Err(ExpressionRewriteDecline::NativeResultAllocation)
        );
    }

    #[test]
    fn native_numeric_inputs_do_not_license_selecting_the_result_object() {
        let source = "set x [expr {4}]; set result [expr {$x + 0}]";
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let owner = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = owner.commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            source, registry, false, profile,
        );
        let mut context = crate::optimiser::PassContext::new(
            source,
            crate::interprocedural::InterproceduralAnalysis::default(),
        );
        context.registry = Some(registry);
        context.dialect = Some(profile);
        context.ir_module = Some(&unit.ir_module);
        let (expression, span) = unit
            .ir_module
            .top_level
            .statements
            .iter()
            .find_map(|statement| match statement {
                crate::ir::Statement::AssignExpr {
                    name, expr, span, ..
                } if name == "result" => Some((expr, *span)),
                _ => None,
            })
            .expect("actual native expression producer");
        let bindings = ExpressionMathBindings::for_module_statement(&unit.ir_module, span)
            .expect("retained whole-expression mapping");
        let actual = context.fold_policy().preparation_context().unwrap();
        assert!(bindings.preparation_for_context(&actual).is_some());
        assert!(
            bindings
                .source_numeric_operands(&actual, registry)
                .is_some(),
            "native numeric operand must survive lowerer source proof retention: {}",
            unit.ir_module
                .top_level
                .statements
                .iter()
                .filter_map(|statement| {
                    statement.tokens().map(|tokens| {
                        tokens
                            .variable_accesses
                            .iter()
                            .map(|access| {
                                (
                                    access.source.span,
                                    access.original_spelling.as_str(),
                                    crate::native_numeric::source_read_operand(access, registry)
                                        .is_some(),
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .map(|reads| format!("{reads:?}"))
                .collect::<Vec<_>>()
                .join("; ")
        );
        for proposed in ["$x", "4"] {
            assert_eq!(
                context.expression_rewrite_equivalence_at(expression, proposed, &Env::new(), span,),
                Err(ExpressionRewriteDecline::NativeResultAllocation)
            );
        }
    }

    #[test]
    fn original_object_conversions_survive_constant_and_strength_rewrites() {
        let policy = FoldPolicy::default().with_invocation_dialect(
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        let environment = Env::from([("x".into(), EnvValue::Int(3))]);
        for (original, proposed) in [("$x+0", "3"), ("$x*0", "0"), ("$x**2", "$x*$x")] {
            let original = crate::expr_parser::parse_expr(original, None);
            assert_eq!(
                expression_rewrite_equivalence(
                    &original,
                    proposed,
                    &environment,
                    policy,
                    None,
                    None,
                ),
                Err(ExpressionRewriteDecline::RetainedObjectCoercion)
            );
        }
        let literal = crate::expr_parser::parse_expr("2+0", None);
        assert_eq!(
            expression_rewrite_equivalence(&literal, "2", &Env::new(), policy, None, None,),
            Ok(())
        );
        assert_eq!(
            expression_rewrite_equivalence(&literal, "3", &Env::new(), policy, None, None,),
            Err(ExpressionRewriteDecline::ValueChanged)
        );
    }

    fn witness(
        version: &str,
        expression: &str,
    ) -> std::sync::Arc<tcl_registry::runtime_expr_validation::PreparedExpressionWitness> {
        let source = format!("set result [expr {{{expression}}}]");
        let owner = tcl_registry::model::ingress::static_context_for(version);
        let registry = owner.commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            &source,
            registry,
            false,
            registry.profile().unwrap(),
        );
        let crate::ir::Statement::AssignExpr { span, .. } = &unit.ir_module.top_level.statements[0]
        else {
            panic!("original retained expression {expression}");
        };
        let bindings =
            ExpressionMathBindings::for_module_statement(&unit.ir_module, *span).unwrap();
        bindings
            .preparation_for_context(
                &FoldPolicy::from_registry(registry)
                    .preparation_context()
                    .unwrap(),
            )
            .unwrap()
            .witness
            .clone()
    }

    // tclsh 8.6.18 and 9.0.4, `tcl::unsupported::representation [expr {E}]`
    // with `set a 1`: every arithmetic E below is "a int ... no string
    // representation"; `$a * 1.0` is a double; `$a == 1` is an int.
    #[test]
    fn an_integer_arithmetic_result_may_become_its_literal() {
        use crate::tcl_expr_eval::TclValue;
        for version in ["tcl8.6", "tcl9.0"] {
            for (expression, value) in [
                ("$a + 2", TclValue::Int(3)),
                ("$a - 2", TclValue::Int(-1)),
                ("$a * 2", TclValue::Int(2)),
                ("$a / 2", TclValue::Int(0)),
                ("$a ** 2", TclValue::Int(1)),
                ("0x10 + $a", TclValue::Int(17)),
                ("0o10 + $a", TclValue::Int(9)),
                (
                    "$a ** 64 + 2 ** 64",
                    TclValue::Big(num_bigint::BigInt::from(2u8).pow(64) + 1u8),
                ),
            ] {
                assert_eq!(
                    contents_only_result_equivalence(&witness(version, expression), &value),
                    Ok(()),
                    "{version} {expression}"
                );
            }
        }
    }

    #[test]
    fn a_double_comparison_or_normalised_result_keeps_its_operation() {
        use crate::tcl_expr_eval::TclValue;
        for version in ["tcl8.6", "tcl9.0"] {
            // 8.6: with `tcl_precision 3` set before its string is generated,
            // `expr {$a / 3.0}` reads back as 0.333 and the literal as
            // 0.3333333333333333.
            for (expression, value) in [
                ("$a / 3.0", TclValue::Float(1.0 / 3.0)),
                ("1.0 + $a", TclValue::Float(2.0)),
                ("$a == 1", TclValue::Int(1)),
                ("$a < 2", TclValue::Int(1)),
                ("!$a", TclValue::Int(0)),
                ("$a", TclValue::Int(1)),
            ] {
                assert_eq!(
                    contents_only_result_equivalence(&witness(version, expression), &value),
                    Err(ExpressionRewriteDecline::NativeResultAllocation),
                    "{version} {expression}"
                );
            }
        }
    }

    #[test]
    fn a_contents_only_replacement_follows_the_result_recipe() {
        for version in ["tcl8.6", "tcl9.0"] {
            // Pooled: C precomputes the constant tree into a literal.
            assert_eq!(
                expression_result_protocol_equivalence(&witness(version, "2 + 2"), None),
                Ok(())
            );
            // Integer-only operators.
            for expression in ["$a % 3", "$a << 1", "$a & 1", "~$a"] {
                assert_eq!(
                    expression_result_protocol_equivalence(&witness(version, expression), None),
                    Ok(()),
                    "{version} {expression}"
                );
            }
            // A numeric or normalised result needs its evaluated value.
            for expression in ["$a + 2", "$a"] {
                assert_eq!(
                    expression_result_protocol_equivalence(&witness(version, expression), None),
                    Err(ExpressionRewriteDecline::NativeResultAllocation),
                    "{version} {expression}"
                );
            }
        }
    }
}
