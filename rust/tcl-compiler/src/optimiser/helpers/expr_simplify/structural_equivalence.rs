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

//! Whether a proposed rewrite is the original expression for every operand value: the same result or the same error.

use super::NumericCtx;
use crate::expr_ast::{BinOp, ExprNode, render_expr};
use crate::expr_parser::parse_expr_for_profile;
use crate::tcl_expr_eval::{FoldPolicy, leading_zero_is_octal};

/// Whether `proposed` is `original` for every operand value: the same result,
/// or the same error. Two licences, each from tclsh 8.6.18 and 9.0.4 run over
/// integers, doubles, `NaN`, words, boolean words and the empty string:
///
/// * any operands: `!(L == R)` and `L != R` (and the `eq`/`ne`, `in`/`ni`
///   pairs and the 9.0 string orderings, each way round), `!!C` and `C` for a
///   comparison `C`, and `L == "w"`
///   with `L eq "w"` for a word `w` that is no number. `!($a && $b)` with
///   `!$a || !$b` is not one: a word operand raises `expected boolean value`
///   in the first and `can't use non-numeric string as operand of "!"` in the
///   second. Nor is `!($a < 1)` with `$a >= 1`, which differ for `NaN`.
/// * integer operands only: every variable is typed an integer and every
///   literal is one, so no operand can raise a conversion error, and the two
///   expressions agree on a grid of integer values, zero and negatives
///   included, so a dropped division or shift is caught.
#[must_use]
pub fn structural_rewrite_equivalence(
    original: &ExprNode,
    proposed: &str,
    numeric: NumericCtx<'_>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> bool {
    structural_equivalence(original, proposed, numeric, dialect, false)
}

/// [`structural_rewrite_equivalence`] for a branch condition, where only the
/// truth of the result is observed.
#[must_use]
pub fn structural_condition_equivalence(
    original: &ExprNode,
    proposed: &str,
    numeric: NumericCtx<'_>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> bool {
    structural_equivalence(original, proposed, numeric, dialect, true)
}

fn structural_equivalence(
    original: &ExprNode,
    proposed: &str,
    numeric: NumericCtx<'_>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    truth_only: bool,
) -> bool {
    let Some(profile) = dialect else {
        return false;
    };
    // Measured on C Tcl only. Jim's numeric comparison rewrites its operand's
    // string, and the F5 dialects have no interpreter to run here.
    if !matches!(
        tcl_syntax::expr::parser::ExprParseContext::for_profile(profile).native_syntax,
        tcl_syntax::expr::parser::NativeExprSyntax::Tcl(_)
    ) {
        return false;
    }
    let proposed = parse_expr_for_profile(proposed, dialect);
    if matches!(proposed, ExprNode::Raw { .. }) {
        return false;
    }
    equivalent_for_any_operand(original, &proposed, profile)
        || equivalent_for_integer_operands(original, &proposed, numeric, profile, truth_only)
}

fn equivalent_for_any_operand(
    original: &ExprNode,
    proposed: &ExprNode,
    profile: &'static tcl_dialect::DialectProfile,
) -> bool {
    use crate::expr_ast::UnaryOp;
    match (original, proposed) {
        (
            ExprNode::Unary {
                op: UnaryOp::Not,
                operand,
            },
            _,
        ) => {
            negates(operand, proposed)
                || matches!(
                    operand.as_ref(),
                    ExprNode::Unary { op: UnaryOp::Not, operand: inner }
                        if truth_valued(inner) && same_text(inner, proposed)
                )
        }
        (
            ExprNode::Binary { op, left, right },
            ExprNode::Binary {
                op: string_op,
                left: new_left,
                right: new_right,
            },
        ) if matches!(
            (op, string_op),
            (BinOp::Eq, BinOp::StrEq) | (BinOp::Ne, BinOp::StrNe)
        ) =>
        {
            same_text(left, new_left)
                && same_text(right, new_right)
                && (word_is_no_number(left, profile) || word_is_no_number(right, profile))
        }
        // `&&` and `||` never evaluate the right operand once the left one
        // decides, so no read or conversion of it is lost.
        (
            ExprNode::Binary {
                op: op @ (BinOp::And | BinOp::Or),
                left,
                ..
            },
            ExprNode::Literal { text, .. },
        ) => integer_literal(left, profile).is_some_and(|decider| match op {
            BinOp::And => decider == 0 && text == "0",
            _ => decider != 0 && text == "1",
        }),
        // A constant condition selects one arm; the other is never evaluated
        // and the selected operand is converted the way a lone operand is.
        (
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            },
            selected,
        ) => integer_literal(condition, profile).is_some_and(|decider| {
            same_text(
                if decider == 0 {
                    false_branch
                } else {
                    true_branch
                },
                selected,
            )
        }),
        _ => false,
    }
}

fn same_text(left: &ExprNode, right: &ExprNode) -> bool {
    render_expr(left) == render_expr(right)
}

/// A comparison, or `&&`/`||` over comparisons: evaluating it cannot raise a
/// conversion error and its result is `0` or `1`.
fn truth_valued(node: &ExprNode) -> bool {
    match node {
        ExprNode::Binary {
            op: BinOp::And | BinOp::Or,
            left,
            right,
        } => truth_valued(left) && truth_valued(right),
        ExprNode::Binary { op, .. } => matches!(
            op,
            BinOp::Eq
                | BinOp::Ne
                | BinOp::Lt
                | BinOp::Le
                | BinOp::Gt
                | BinOp::Ge
                | BinOp::StrEq
                | BinOp::StrNe
                | BinOp::In
                | BinOp::Ni
        ),
        _ => false,
    }
}

/// Whether `proposed` is `!positive` for every operand value: the written
/// negation itself, the complementary operator over the same operands, or De
/// Morgan over truth-valued sides (which keeps the short circuit).
fn negates(positive: &ExprNode, proposed: &ExprNode) -> bool {
    use crate::expr_ast::UnaryOp;
    if let ExprNode::Unary {
        op: UnaryOp::Not,
        operand,
    } = proposed
        && same_text(positive, operand)
    {
        return true;
    }
    let (
        ExprNode::Binary { op, left, right },
        ExprNode::Binary {
            op: negated,
            left: new_left,
            right: new_right,
        },
    ) = (positive, proposed)
    else {
        return false;
    };
    match (op, negated) {
        (BinOp::Eq, BinOp::Ne)
        | (BinOp::Ne, BinOp::Eq)
        | (BinOp::StrEq, BinOp::StrNe)
        | (BinOp::StrNe, BinOp::StrEq)
        | (BinOp::In, BinOp::Ni)
        | (BinOp::Ni, BinOp::In)
        | (BinOp::StrLt, BinOp::StrGe)
        | (BinOp::StrGe, BinOp::StrLt)
        | (BinOp::StrLe, BinOp::StrGt)
        | (BinOp::StrGt, BinOp::StrLe) => same_text(left, new_left) && same_text(right, new_right),
        (BinOp::And, BinOp::Or) | (BinOp::Or, BinOp::And) => {
            truth_valued(left)
                && truth_valued(right)
                && negates(left, new_left)
                && negates(right, new_right)
        }
        _ => false,
    }
}

fn integer_literal(node: &ExprNode, profile: &'static tcl_dialect::DialectProfile) -> Option<i64> {
    let ExprNode::Literal { text, .. } = node else {
        return None;
    };
    match tcl_syntax::number::parse_whole_with(
        text.trim(),
        tcl_syntax::number::ParseFlags::for_syntax(profile.grammar.numbers),
    )? {
        tcl_syntax::number::Number::Int(value) => Some(value),
        _ => None,
    }
}

/// A fixed string operand that no numeric conversion accepts, so `==` against
/// it compares strings.
fn word_is_no_number(node: &ExprNode, profile: &'static tcl_dialect::DialectProfile) -> bool {
    let ExprNode::String { text, .. } = node else {
        return false;
    };
    tcl_syntax::expr::fixed_string_operand(text).is_some_and(|word| {
        tcl_syntax::number::parse_whole_with(
            word.trim(),
            tcl_syntax::number::ParseFlags::for_syntax(profile.grammar.numbers),
        )
        .is_none()
    })
}

fn equivalent_for_integer_operands(
    original: &ExprNode,
    proposed: &ExprNode,
    numeric: NumericCtx<'_>,
    profile: &'static tcl_dialect::DialectProfile,
    truth_only: bool,
) -> bool {
    const GRID: [i64; 7] = [-3, -1, 0, 1, 2, 7, 1 << 40];
    let Some(types) = numeric else {
        return false;
    };
    let mut names: Vec<&str> = Vec::new();
    for tree in [original, proposed] {
        for node in tree.evaluation_nodes() {
            match node {
                ExprNode::Var { text, name, .. } => {
                    if !types.integer.contains(text)
                        || text.strip_prefix('$') != Some(name.as_str())
                    {
                        return false;
                    }
                    if !names.contains(&name.as_str()) {
                        names.push(name);
                    }
                }
                ExprNode::Literal { text, .. } => {
                    if !matches!(
                        tcl_syntax::number::parse_whole_with(
                            text.trim(),
                            tcl_syntax::number::ParseFlags::for_syntax(profile.grammar.numbers),
                        ),
                        Some(tcl_syntax::number::Number::Int(_))
                    ) {
                        return false;
                    }
                }
                ExprNode::Unary { .. } | ExprNode::Binary { .. } | ExprNode::Ternary { .. } => {}
                _ => return false,
            }
        }
    }
    if names.is_empty() || names.len() > 4 {
        return false;
    }
    let policy = FoldPolicy::for_profile(leading_zero_is_octal(profile), Some(profile));
    let value = |tree: &ExprNode, env: &crate::tcl_expr_eval::Env| {
        crate::tcl_expr_eval::analyse_tcl_expr_with_resolved_math_bindings(
            tree,
            env,
            policy,
            &|_, _| None,
            None,
        )
        .and_then(|evaluation| {
            crate::tcl_expr_eval::format_tcl_value_with_policy(&evaluation.value, policy)
        })
        .map(|text| {
            if truth_only {
                (text != "0").to_string()
            } else {
                text
            }
        })
    };
    let mut indices = vec![0usize; names.len()];
    loop {
        let env: crate::tcl_expr_eval::Env = names
            .iter()
            .zip(&indices)
            .map(|(name, &index)| {
                (
                    (*name).to_owned(),
                    crate::tcl_expr_eval::EnvValue::Int(GRID[index]),
                )
            })
            .collect();
        if value(original, &env) != value(proposed, &env) {
            return false;
        }
        let mut position = 0;
        loop {
            if position == indices.len() {
                return true;
            }
            indices[position] += 1;
            if indices[position] < GRID.len() {
                break;
            }
            indices[position] = 0;
            position += 1;
        }
    }
}
