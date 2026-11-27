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

//! Conditional expression grammar and normal result representation.
//!
//! This owner supplies no compiler entry, fixed function table, completion,
//! object identity or native header. A source consumer must retain the original
//! handler, input occurrences and the separate current literal-pool evidence.

use crate::TclType;
use tcl_syntax::expr::mathfunc::MathResultClass;
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
        dialect: tcl_native_model::InvocationDialect,
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
        variable: impl FnMut(&ExprNode) -> Option<TclType>,
    ) -> Option<TclType> {
        self.normal_result_representation_with_functions(pool, variable, |_| None)
    }

    /// [`Self::normal_result_representation`] where `function` answers the
    /// result class of a call whose dispatch the caller has proved. A call it
    /// does not answer for has no category.
    #[must_use]
    pub fn normal_result_representation_with_functions(
        &self,
        pool: ConditionalExpressionPoolState,
        mut variable: impl FnMut(&ExprNode) -> Option<TclType>,
        mut function: impl FnMut(&ExprNode) -> Option<MathResultClass>,
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
        self.node_result(&self.tree, &mut variable, &mut function, 0)
    }

    fn binary_result(
        &self,
        op: BinOp,
        left: &ExprNode,
        right: &ExprNode,
        variable: &mut impl FnMut(&ExprNode) -> Option<TclType>,
        function: &mut impl FnMut(&ExprNode) -> Option<MathResultClass>,
        next: usize,
    ) -> Option<TclType> {
        match op {
            BinOp::LShift | BinOp::RShift | BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor => {
                Some(TclType::Int)
            }
            BinOp::Mod => {
                let left = self.node_result(left, variable, function, next);
                let right = self.node_result(right, variable, function, next);
                (!matches!(left, Some(TclType::Double)) && !matches!(right, Some(TclType::Double)))
                    .then_some(TclType::Int)
            }
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Pow => {
                let left = self
                    .node_result(left, variable, function, next)
                    .map(truth_value_as_integer);
                let right = self
                    .node_result(right, variable, function, next)
                    .map(truth_value_as_integer);
                match (left, right) {
                    (Some(TclType::Int), Some(TclType::Int)) => Some(TclType::Int),
                    (Some(TclType::Double), _) | (_, Some(TclType::Double)) => {
                        Some(TclType::Double)
                    }
                    _ => Some(TclType::Numeric),
                }
            }
            // A comparison, a membership test and a short-circuit
            // operator complete normally only with 0 or 1: 8.4 makes a
            // fresh integer or pushes the literal `0`/`1`, 8.5 and later
            // return the interpreter's shared 0/1 constants. That is the
            // value, whatever becomes of the shared object's intrep.
            BinOp::And
            | BinOp::Or
            | BinOp::Eq
            | BinOp::Ne
            | BinOp::Lt
            | BinOp::Le
            | BinOp::Gt
            | BinOp::Ge
            | BinOp::StrEq
            | BinOp::StrNe
            | BinOp::StrLt
            | BinOp::StrLe
            | BinOp::StrGt
            | BinOp::StrGe
            | BinOp::In
            | BinOp::Ni => Some(TclType::Boolean),
            _ => None,
        }
    }

    fn node_result(
        &self,
        node: &ExprNode,
        variable: &mut impl FnMut(&ExprNode) -> Option<TclType>,
        function: &mut impl FnMut(&ExprNode) -> Option<MathResultClass>,
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
            // A variable holding a truth value keeps it: `expr {$flag}`
            // returns the word it read.
            ExprNode::Var { .. } => variable(node).filter(|kind| {
                matches!(
                    kind,
                    TclType::Int | TclType::Double | TclType::Numeric | TclType::Boolean
                )
            }),
            ExprNode::Unary { op, operand, .. } => match op {
                UnaryOp::Pos | UnaryOp::Neg => self
                    .node_result(operand, variable, function, next)
                    .map(truth_value_as_integer)
                    .filter(|kind| {
                        matches!(kind, TclType::Int | TclType::Double | TclType::Numeric)
                    }),
                UnaryOp::BitNot => Some(TclType::Int),
                // `!` leaves 0 or 1 in every C Tcl release.
                UnaryOp::Not => Some(TclType::Boolean),
                UnaryOp::WordNot => None,
            },
            ExprNode::Binary {
                op, left, right, ..
            } => self.binary_result(*op, left, right, variable, function, next),
            // The selected arm's value is the result, so two arms of one
            // category give that category and an integer with a double gives
            // a number.
            ExprNode::Ternary {
                true_branch,
                false_branch,
                ..
            } => {
                let chosen = self.node_result(true_branch, variable, function, next)?;
                let other = self.node_result(false_branch, variable, function, next)?;
                selected_arm_result(chosen, other)
            }
            ExprNode::Call { args, .. } => match function(node)? {
                MathResultClass::Int => Some(TclType::Int),
                MathResultClass::Float => Some(TclType::Double),
                MathResultClass::Bool => Some(TclType::Boolean),
                // The result is one of the operands, each a number.
                MathResultClass::Numeric => {
                    let mut joined = None;
                    for argument in args {
                        let kind = self
                            .node_result(argument, variable, function, next)
                            .map(truth_value_as_integer);
                        joined = Some(numeric_operand_join(joined, kind));
                    }
                    Some(joined.unwrap_or(TclType::Numeric))
                }
                MathResultClass::Any => None,
            },
            // A script result has independent dispatch alternatives.
            _ => None,
        }
    }
}

/// The selected arm's value is the result, so two arms of one category give
/// that category and an integer with a double gives a number.
fn selected_arm_result(chosen: TclType, other: TclType) -> Option<TclType> {
    let numeric = |kind| matches!(kind, TclType::Int | TclType::Double | TclType::Numeric);
    if chosen == other {
        Some(chosen)
    } else if numeric(chosen) && numeric(other) {
        Some(TclType::Numeric)
    } else {
        None
    }
}

/// Join one more numeric operand into the category of a result that is one
/// of its operands. An operand of no known category is still a number.
fn numeric_operand_join(joined: Option<TclType>, operand: Option<TclType>) -> TclType {
    let kind = match operand {
        Some(kind @ (TclType::Int | TclType::Double)) => kind,
        _ => TclType::Numeric,
    };
    match joined {
        Some(previous) if previous != kind => TclType::Numeric,
        _ => kind,
    }
}

/// An arithmetic operator reads a 0/1 truth value as that integer.
const fn truth_value_as_integer(kind: TclType) -> TclType {
    match kind {
        TclType::Boolean => TclType::Int,
        other => other,
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

    /// A comparison, membership test, short-circuit operator and `!` give 0
    /// or 1 in every release that parses them. tclsh 8.6.18 and 9.0.4, with
    /// `set a 1; set b 0`: `expr {$a < $b}` is 0, `expr {$a || $b}` is 1,
    /// `expr {!$a}` is 0, `expr {$a in $b}` is 0, `expr {($a<$b)+1}` is 1 and
    /// `expr {$a ? ($a>0) : ($b<0)}` is 1. 8.4 pushes `Tcl_NewLongObj(iResult)`
    /// for a comparison and the literal `0`/`1` for `&&`/`||`
    /// (`tclExecute.c`, `CompileLandOrLorExpr`).
    #[test]
    fn conditional_expression_truth_operators_give_a_boolean_in_every_release() {
        let result = |profile: &str, source: &str| {
            ConditionalExpressionEvaluation::prepare(source, &context(profile)).map(|evaluation| {
                evaluation
                    .normal_result_representation(ConditionalExpressionPoolState::Unknown, |_| {
                        Some(TclType::Int)
                    })
            })
        };
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for source in [
                "$a < $b",
                "$a <= $b",
                "$a > $b",
                "$a >= $b",
                "$a == $b",
                "$a != $b",
                "$a eq $b",
                "$a ne $b",
                "$a && $b",
                "$a || $b",
                "!$a",
                "$a < $b == $a > $b",
                "$c ? ($a > 0) : ($b < 0)",
            ] {
                assert_eq!(
                    result(profile, source),
                    Some(Some(TclType::Boolean)),
                    "{profile}: {source}"
                );
            }
            for (source, expected) in [
                ("($a < $b) + 1", TclType::Int),
                ("-($a < $b)", TclType::Int),
                ("$c ? $a : $b", TclType::Int),
                ("$c ? 1.0 : 2.0", TclType::Double),
                ("$c ? $a : 2.0", TclType::Numeric),
            ] {
                assert_eq!(
                    result(profile, source),
                    Some(Some(expected)),
                    "{profile}: {source}"
                );
            }
            // One arm a truth value and the other a number: neither category.
            assert_eq!(
                result(profile, "$c ? ($a < $b) : 2.5"),
                Some(None),
                "{profile}"
            );
            // `in`/`ni` arrive in 8.5 and the string orderings in 9.0.
            let membership = profile != "tcl8.4";
            let ordering = matches!(profile, "tcl9.0" | "tcl9.1");
            for (source, parses) in [
                ("$a in $b", membership),
                ("$a ni $b", membership),
                ("$a lt $b", ordering),
                ("$a ge $b", ordering),
            ] {
                assert_eq!(
                    result(profile, source),
                    parses.then_some(Some(TclType::Boolean)),
                    "{profile}: {source}"
                );
            }
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
