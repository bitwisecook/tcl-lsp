// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional expression grammar and normal result representation.
//!
//! This owner supplies no compiler entry, fixed function table, completion,
//! object identity or native header. A source consumer must retain the original
//! handler, input occurrences and the separate current literal-pool evidence.

use crate::TclType;
use tcl_syntax::expr::parser::{CheckedExprParse, ExprParseContext, NativeExprSyntax};
use tcl_syntax::expr::{BinOp, ExprNode, UnaryOp};

/// Checked original expression, independent of actual compiler preparation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConditionalExpressionEvaluation {
    tree: ExprNode,
    context: ExprParseContext,
}

/// Current symbolic pool state, independently of mathematical result category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConditionalExpressionPoolState {
    /// A fresh authored pool with no intervening representation-changing use.
    InitialAuthoredPool,
    /// No current representation evidence for an object reused from the pool.
    Unknown,
}

impl ConditionalExpressionEvaluation {
    /// Validate exact bytes under the original dialect's checked grammar.
    /// Fixed-function registration and nested script preparation remain absent.
    #[must_use]
    pub fn prepare(source: &str, context: &ExprParseContext) -> Option<Self> {
        let CheckedExprParse::Parsed(tree) =
            tcl_syntax::expr::parser::parse_expr_checked_with_context(source, context)
        else {
            return None;
        };
        Some(Self {
            tree,
            context: *context,
        })
    }

    /// Original checked operand topology for conditional read/call inventories.
    #[must_use]
    pub const fn tree(&self) -> &ExprNode {
        &self.tree
    }

    /// Construct the native implicit command-name value from an exact checked
    /// function occurrence. Fixed-table engines decline; no command-head word,
    /// registration, runtime lookup or evaluation capability is created.
    #[must_use]
    pub fn original_function_command_name(
        &self,
        source: &str,
        ordinal: usize,
        dialect: crate::InvocationDialect,
    ) -> Option<crate::mathfunc::NativeExpressionFunctionCommandName> {
        // Proof: naming.expression.original-function-navigation
        // docs/design/analysis/name-resolution-proofs/original-function-navigation.md
        crate::mathfunc::NativeExpressionFunctionCommandName::from_original_expression(
            self, source, ordinal, dialect,
        )
    }

    /// Original lexical grammar retained by the checked expression owner.
    /// This does not select a current runtime provider or compiler protocol.
    #[must_use]
    pub const fn lexer_grammar(&self) -> tcl_dialect::LexerGrammar {
        self.context.lexer_grammar
    }

    /// Original arithmetic normalization on the inner normal path. This
    /// supplies no invocation, compiler entry, totality or object receipt.
    #[must_use]
    pub fn numeric_reentry_is_idempotent(&self, inner: &Self) -> bool {
        crate::runtime_expr_validation::checked_numeric_reentry_is_idempotent(
            &self.tree,
            &self.context,
            &inner.tree,
            &inner.context,
        )
    }

    /// Original checked preparation axes, independent of an actual entry.
    #[must_use]
    pub const fn context(&self) -> &ExprParseContext {
        &self.context
    }

    /// Result representation on a normal path with original operand evidence.
    /// Unknown functions/scripts never borrow catalogue return types. Constant
    /// operator results require a separate current authored pool receipt.
    #[must_use]
    pub fn normal_result_representation(
        &self,
        pool: ConditionalExpressionPoolState,
        mut variable: impl FnMut(&ExprNode) -> Option<TclType>,
    ) -> Option<TclType> {
        if !matches!(self.context.native_syntax, NativeExprSyntax::Tcl(_)) {
            return None;
        }
        if crate::runtime_expr_validation::selected_numeric_result_recipe(
            &self.tree,
            self.context.native_syntax,
        ) == crate::runtime_expr_validation::NativeExpressionResultRecipe::PooledValue
            && pool != ConditionalExpressionPoolState::InitialAuthoredPool
        {
            return None;
        }
        self.node_result(&self.tree, &mut variable, 0)
    }

    fn node_result(
        &self,
        node: &ExprNode,
        variable: &mut impl FnMut(&ExprNode) -> Option<TclType>,
        depth: usize,
    ) -> Option<TclType> {
        if depth > 128 {
            return None;
        }
        let next = depth + 1;
        match node {
            ExprNode::Literal { text, .. } => {
                use tcl_syntax::number::{Number, ParseFlags};
                match tcl_syntax::number::parse_whole_with(
                    text.trim(),
                    ParseFlags::for_syntax(self.context.lexer_grammar.numbers),
                ) {
                    Some(Number::Int(_) | Number::Big { .. }) => Some(TclType::Int),
                    Some(Number::Double(_) | Number::Nan { .. }) => Some(TclType::Double),
                    None => Some(TclType::String),
                }
            }
            ExprNode::Var { .. } => variable(node)
                .filter(|kind| matches!(kind, TclType::Int | TclType::Double | TclType::Numeric)),
            ExprNode::Unary { op, operand, .. } => match op {
                UnaryOp::Pos | UnaryOp::Neg => {
                    self.node_result(operand, variable, next).filter(|kind| {
                        matches!(kind, TclType::Int | TclType::Double | TclType::Numeric)
                    })
                }
                UnaryOp::BitNot => Some(TclType::Int),
                UnaryOp::Not | UnaryOp::WordNot => None,
            },
            ExprNode::Binary {
                op, left, right, ..
            } => match op {
                BinOp::LShift | BinOp::RShift | BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor => {
                    Some(TclType::Int)
                }
                BinOp::Mod => {
                    let left = self.node_result(left, variable, next);
                    let right = self.node_result(right, variable, next);
                    (!matches!(left, Some(TclType::Double))
                        && !matches!(right, Some(TclType::Double)))
                    .then_some(TclType::Int)
                }
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Pow => {
                    let left = self.node_result(left, variable, next);
                    let right = self.node_result(right, variable, next);
                    match (left, right) {
                        (Some(TclType::Int), Some(TclType::Int)) => Some(TclType::Int),
                        (Some(TclType::Double), _) | (_, Some(TclType::Double)) => {
                            Some(TclType::Double)
                        }
                        _ => Some(TclType::Numeric),
                    }
                }
                _ => None,
            },
            // Forwarded branches and script/function results have independent
            // result-object and dispatch alternatives.
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(profile: &str) -> ExprParseContext {
        ExprParseContext::for_profile(
            crate::model::ingress::static_context_for(profile)
                .commands()
                .profile()
                .unwrap(),
        )
    }

    #[test]
    fn conditional_expression_numeric_pool_and_operand_normalisation_are_distinct() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let parser = context(profile);
            let literal = ConditionalExpressionEvaluation::prepare("3.5", &parser).unwrap();
            assert_eq!(
                literal.normal_result_representation(
                    ConditionalExpressionPoolState::InitialAuthoredPool,
                    |_| { None }
                ),
                Some(TclType::Double)
            );
            let pooled = ConditionalExpressionEvaluation::prepare("7 << 1", &parser).unwrap();
            assert_eq!(
                pooled.normal_result_representation(
                    ConditionalExpressionPoolState::InitialAuthoredPool,
                    |_| None
                ),
                Some(TclType::Int)
            );
            assert_eq!(
                pooled
                    .normal_result_representation(ConditionalExpressionPoolState::Unknown, |_| {
                        None
                    }),
                (profile == "tcl8.4").then_some(TclType::Int)
            );
            let variable = ConditionalExpressionEvaluation::prepare("$view", &parser).unwrap();
            assert_eq!(
                variable
                    .normal_result_representation(ConditionalExpressionPoolState::Unknown, |_| {
                        Some(TclType::Double)
                    }),
                Some(TclType::Double)
            );
            assert!(
                variable
                    .normal_result_representation(ConditionalExpressionPoolState::Unknown, |_| None)
                    .is_none()
            );
        }
    }

    #[test]
    fn conditional_expression_integer_power_and_modulo_follow_normal_native_paths() {
        for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let parser = context(profile);
            for source in [
                "$base ** $exponent",
                "$base % $exponent",
                "$base / $exponent",
            ] {
                let evaluation = ConditionalExpressionEvaluation::prepare(source, &parser).unwrap();
                assert_eq!(
                    evaluation.normal_result_representation(
                        ConditionalExpressionPoolState::Unknown,
                        |_| Some(TclType::Int),
                    ),
                    Some(TclType::Int),
                    "{profile}: {source}"
                );
            }
            let evaluation =
                ConditionalExpressionEvaluation::prepare("$floating % 3", &parser).unwrap();
            assert!(
                evaluation
                    .normal_result_representation(
                        ConditionalExpressionPoolState::Unknown,
                        |_| Some(TclType::Double),
                    )
                    .is_none()
            );
            let evaluation =
                ConditionalExpressionEvaluation::prepare("$floating / 3", &parser).unwrap();
            assert_eq!(
                evaluation
                    .normal_result_representation(ConditionalExpressionPoolState::Unknown, |_| {
                        Some(TclType::Double)
                    },),
                Some(TclType::Double)
            );
        }
        assert!(ConditionalExpressionEvaluation::prepare("2 ** -1", &context("tcl8.4")).is_none());
    }

    #[test]
    fn conditional_numeric_reentry_preserves_only_fresh_arithmetic_results() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let parser = context(profile);
            let outer =
                ConditionalExpressionEvaluation::prepare("[expr {$x * 2}]", &parser).unwrap();
            let inner = ConditionalExpressionEvaluation::prepare("$x * 2", &parser).unwrap();
            assert_eq!(
                outer.numeric_reentry_is_idempotent(&inner),
                profile != "jim"
            );
            for expression in ["$x", "1 + 2", "abs($x)", "$x + [side_effect]", "$x < 2"] {
                let inner = ConditionalExpressionEvaluation::prepare(expression, &parser).unwrap();
                let expected = profile == "tcl8.4" && expression == "1 + 2";
                assert_eq!(
                    outer.numeric_reentry_is_idempotent(&inner),
                    expected,
                    "{profile}: {expression}"
                );
            }
            let other =
                ConditionalExpressionEvaluation::prepare("$x * 2", &context("tcl9.1")).unwrap();
            assert_eq!(
                outer.numeric_reentry_is_idempotent(&other),
                profile == "tcl9.1"
            );
        }
    }

    #[test]
    fn conditional_expression_calls_and_unknown_dialects_supply_no_header() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let parser = context(profile);
            for source in ["sqrt(4)", "[producer]", "1 ? [producer] : 2"] {
                let evaluation = ConditionalExpressionEvaluation::prepare(source, &parser).unwrap();
                assert!(
                    evaluation
                        .normal_result_representation(
                            ConditionalExpressionPoolState::InitialAuthoredPool,
                            |_| None
                        )
                        .is_none()
                );
            }
        }
        let mut parser = context("tcl9.0");
        parser.native_syntax = NativeExprSyntax::Unknown;
        assert!(ConditionalExpressionEvaluation::prepare("1+2", &parser).is_none());
    }
}
