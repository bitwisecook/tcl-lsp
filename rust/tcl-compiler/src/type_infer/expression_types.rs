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

//! The type an expression node produces: operators and math-function calls.

use std::collections::HashMap;

use tcl_dialect::NumberSyntax;
use tcl_registry::TclType;

use super::literals::expr_literal_type;
use crate::expr_ast::{BinOp, ExprNode, UnaryOp};
use crate::types::{TypeKind, TypeLattice, type_join};
use tcl_source::limits::MAX_EXPR_NODE_DEPTH;

/// Infer the type produced by an expression AST node.
///
/// Numeric operators always produce a numeric type; string comparison
/// operators produce boolean; variable references look up the known
/// type from `var_types`.
#[must_use]
#[cfg(test)]
pub(super) fn infer_expr_type(
    node: &ExprNode,
    var_types: &HashMap<(String, u32, u32), TypeLattice>,
    depth: u32,
    numbers: NumberSyntax,
) -> TypeLattice {
    // Native-stack safety net: `ExprNode` operator trees are
    // walked with one native frame per nesting level. Past the cap, give up
    // conservatively with `overdefined` — the same "any type / can't tell"
    // top this function already returns for opaque `Command`/`Raw` nodes, so
    // no caller narrows a type it shouldn't.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return TypeLattice::overdefined();
    }
    match node {
        ExprNode::Literal { text, .. } => expr_literal_type(text, numbers),

        ExprNode::String { .. } | ExprNode::CompiledWord { .. } => TypeLattice::of(TclType::String),

        ExprNode::Var {
            text, start, end, ..
        } => var_types
            .get(&(text.clone(), *start, *end))
            .cloned()
            .unwrap_or_else(TypeLattice::unknown),

        ExprNode::Binary {
            op, left, right, ..
        } => {
            match op {
                // BITWISE / shift → always Int (Tcl `expr` coerces the
                // operands to integers), never the weaker Numeric that
                // arithmetic yields.
                BinOp::LShift | BinOp::RShift | BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor => {
                    TypeLattice::of(TclType::Int)
                }

                // LOGICAL / COMPARISON → always Boolean.  The
                // six iRules string predicates (`contains` / `starts_with`
                // / `ends_with` / `equals` / `matches_glob` /
                // `matches_regex`) plus the word-logical `and` / `or` used
                // to fall through `_ => overdefined()`.
                BinOp::And
                | BinOp::Or
                | BinOp::WordAnd
                | BinOp::WordOr
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
                | BinOp::Ni
                | BinOp::Contains
                | BinOp::StartsWith
                | BinOp::EndsWith
                | BinOp::StrEquals
                | BinOp::Matches
                | BinOp::MatchesGlob
                | BinOp::MatchesRegex => TypeLattice::of(TclType::Boolean),

                // ARITHMETIC / DIVISION → `_arithmetic_result` over the
                // operand types, but only when both are `Known`;
                // otherwise Numeric.
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod | BinOp::Pow => {
                    let lt = infer_expr_type(left, var_types, depth + 1, numbers);
                    let rt = infer_expr_type(right, var_types, depth + 1, numbers);
                    if lt.kind() == TypeKind::Known && rt.kind() == TypeKind::Known {
                        arithmetic_result(&lt, &rt)
                    } else {
                        TypeLattice::of(TclType::Numeric)
                    }
                }
            }
        }

        ExprNode::Unary { op, operand, .. } => match op {
            // Arithmetic sign is identity (same intrep as the operand);
            // bitwise NOT always coerces to `Int` (`~$double` → Int);
            // logical NOT yields `Boolean`.'
            // `UnaryOpKind` BITWISE arm (`~` was grouped with
            // the identity ops and leaked the operand's `Double`).
            UnaryOp::Neg | UnaryOp::Pos => infer_expr_type(operand, var_types, depth + 1, numbers),
            UnaryOp::BitNot => TypeLattice::of(TclType::Int),
            UnaryOp::Not | UnaryOp::WordNot => TypeLattice::of(TclType::Boolean),
        },

        ExprNode::Ternary {
            true_branch,
            false_branch,
            ..
        } => {
            let tt = infer_expr_type(true_branch, var_types, depth + 1, numbers);
            let ft = infer_expr_type(false_branch, var_types, depth + 1, numbers);
            type_join(&tt, &ft)
        }

        // Math-function calls resolve through the expr-function table
        // — `sqrt($x)` is Double, `int(...)` is Int, etc.
        ExprNode::Call { function, args, .. } => {
            expr_call_type(function, args, var_types, depth, numbers)
        }

        // Command substitutions and raw/unrecognised expression text
        // need the registry (or runtime context) to resolve. Without it
        // we over-approximate to overdefined; the outer
        // evaluate_type_def handles command-sub type resolution where the
        // registry is in scope.
        ExprNode::Command { .. } | ExprNode::Raw { .. } => TypeLattice::overdefined(),
    }
}

/// INT op INT → INT
/// (boolean counts as int), DOUBLE anywhere → DOUBLE, otherwise
/// NUMERIC.  Callers guarantee both operand types are `Known`.
#[cfg(test)]
fn arithmetic_result(lt: &TypeLattice, rt: &TypeLattice) -> TypeLattice {
    match (lt.tcl_type(), rt.tcl_type()) {
        (Some(TclType::Int | TclType::Boolean), Some(TclType::Int | TclType::Boolean)) => {
            TypeLattice::of(TclType::Int)
        }
        (Some(TclType::Double), _) | (_, Some(TclType::Double)) => TypeLattice::of(TclType::Double),
        _ => TypeLattice::of(TclType::Numeric),
    }
}

/// Resolve a Tcl `expr` math-function call to its result type.
///
/// `abs` is identity
/// (preserves its operand's type), `max` / `min` join their operand
/// types, every other built-in returns its declared type, and an
/// unknown function is conservatively `Numeric` (an `expr` function
/// always yields a number).
#[cfg(test)]
pub(super) fn expr_call_type(
    function: &str,
    args: &[ExprNode],
    var_types: &HashMap<(String, u32, u32), TypeLattice>,
    depth: u32,
    numbers: NumberSyntax,
) -> TypeLattice {
    use tcl_syntax::expr::mathfunc::{MathResultClass, result_class};
    match result_class(function) {
        MathResultClass::Int => TypeLattice::of(TclType::Int),
        MathResultClass::Float => TypeLattice::of(TclType::Double),
        MathResultClass::Bool => TypeLattice::of(TclType::Boolean),
        // `depth` is the level of the enclosing `Call` node; its args are one
        // level deeper; `infer_expr_type` guards the cap itself.
        MathResultClass::Numeric => {
            let mut operands = args
                .iter()
                .map(|arg| infer_expr_type(arg, var_types, depth + 1, numbers));
            match operands.next() {
                Some(first) => operands.fold(first, |acc, next| type_join(&acc, &next)),
                None => TypeLattice::of(TclType::Numeric),
            }
        }
        MathResultClass::Any => TypeLattice::of(TclType::Numeric),
    }
}
