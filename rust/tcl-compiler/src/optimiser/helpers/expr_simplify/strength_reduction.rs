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

//! Strength reduction: ternary, unary, arithmetic, power, modulo, shift, bitwise and logical identities.

use super::numeric_proofs::{node_cannot_be_nan, node_provably_integer, node_provably_numeric};
use super::{NumericCtx, expr_has_command_subst, int_literal_value, make_int_literal};
use crate::expr_ast::{BinOp, ExprNode, render_expr};

/// Strength-reduce a single expression text.
///
/// Parses `expr`, applies the strength-reduction rewrites, and
/// re-renders. Returns `(text, changed)` — where `changed`
/// indicates a text change. Unparseable / command-subst-laden
/// inputs come back unchanged.
#[must_use]
#[cfg(test)]
pub fn try_strength_reduce_expr(expr: &str) -> (String, bool) {
    try_strength_reduce_expr_typed(expr, None, None)
}

/// As [`try_strength_reduce_expr`], but with a numeric-type context for the
/// operand-dropping identities. See [`NumericCtx`].
#[must_use]
pub fn try_strength_reduce_expr_typed(
    expr: &str,
    numeric: NumericCtx<'_>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> (String, bool) {
    let trimmed = expr.trim();
    let parsed = crate::expr_parser::parse_expr_for_profile(trimmed, dialect);
    if matches!(parsed, ExprNode::Raw { .. }) || expr_has_command_subst(&parsed) {
        return (expr.to_owned(), false);
    }
    // A standalone strength-reduce has no enclosing boolean context — the
    // expression's full value is consumed.
    let Some(rewritten) = strength_reduce_node(&parsed, false, numeric) else {
        return (expr.to_owned(), false);
    };
    let rendered = render_expr(&rewritten);
    let changed = rendered != trimmed;
    (rendered, changed)
}

// AST-level rewriters (private helpers)

/// One pass of strength reduction. Returns `None` when no
/// rewrite applies. Conservative — only obviously-safe rewrites
/// (no overflow / divide-by-zero concerns).
pub(super) fn strength_reduce_node(
    node: &ExprNode,
    bool_context: bool,
    numeric: NumericCtx<'_>,
) -> Option<ExprNode> {
    match node {
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => reduce_ternary(condition, true_branch, false_branch),
        ExprNode::Unary { op, operand } => reduce_unary(*op, operand, bool_context, numeric),
        ExprNode::Binary { op, left, right } => {
            reduce_binary(*op, left, right, bool_context, numeric)
        }
        _ => None,
    }
}

/// `cond ? a : b` with a constant `cond` collapses to the chosen branch.
fn reduce_ternary(
    condition: &ExprNode,
    true_branch: &ExprNode,
    false_branch: &ExprNode,
) -> Option<ExprNode> {
    let k = int_literal_value(condition)?;
    if k != 0 {
        Some(true_branch.clone())
    } else {
        Some(false_branch.clone())
    }
}

/// Unary identities and negation collapses.
pub(super) fn reduce_unary(
    op: crate::expr_ast::UnaryOp,
    operand: &ExprNode,
    bool_context: bool,
    numeric: NumericCtx<'_>,
) -> Option<ExprNode> {
    use crate::expr_ast::UnaryOp;

    // `+x` → `x` (arithmetic identity) — drops the unary `+`, so `x` must be
    // provably numeric or the coercion error (`expr {+$s}`) would be lost.
    if matches!(op, UnaryOp::Pos) && node_provably_numeric(operand, numeric) {
        return Some(operand.clone());
    }

    // `!!x` → `x` / `not not x` → `x` — only sound when `x` already yields a
    // `0`/`1` boolean or the result is consumed in a boolean context;
    // otherwise the double-negation is the very normalisation that turns a
    // non-`0`/`1` value into `0`/`1` (`expr {!!2}` is `1`).
    if matches!(op, UnaryOp::Not | UnaryOp::WordNot)
        && let ExprNode::Unary {
            op: inner_op,
            operand: inner_operand,
        } = operand
        && matches!(inner_op, UnaryOp::Not | UnaryOp::WordNot)
        && (bool_context || is_boolean_expr(inner_operand))
    {
        return Some((**inner_operand).clone());
    }

    // `~~x` → `x` — drops both bitwise negations, so `x` must be provably
    // numeric or the coercion error would be lost.
    if matches!(op, UnaryOp::BitNot)
        && let ExprNode::Unary {
            op: UnaryOp::BitNot,
            operand: inner_operand,
        } = operand
        && node_provably_numeric(inner_operand, numeric)
    {
        return Some((**inner_operand).clone());
    }

    // `!(x <cmp> y)` → inverted comparison, and DeMorgan for `!(a && b)`.
    // `BinOp::inverse()` (tcl_syntax::expr::operators) is the single source
    // for which comparison inverts to which — a local match here drifts, and
    // one missing the TIP 461 string-ordering four (`lt`/`le`/`gt`/`ge`) or
    // list membership (`in`/`ni`) leaves `!(x lt y)` / `!(x in list)`
    // unsimplified even though the same total-order/negation identity holds
    // for them as for the numeric/string-eq forms already covered.
    //
    // The four *ordered numeric* rows (`<`/`<=`/`>`/`>=`) carry a NaN
    // precondition (`BinOp::inverse_needs_non_nan`): `expr {!(NaN < 1)}` is 1
    // but `expr {NaN >= 1}` is 0, so they fire only when both operands are
    // proved non-NaN. `==`/`!=` are exact complements even for NaN, and the
    // string / membership operators have no NaN rule, so those stay
    // unconditional.
    if matches!(op, UnaryOp::Not | UnaryOp::WordNot)
        && let ExprNode::Binary {
            op: inner_op,
            left,
            right,
        } = operand
    {
        let nan_safe = !inner_op.inverse_needs_non_nan()
            || (node_cannot_be_nan(left, numeric) && node_cannot_be_nan(right, numeric));
        if nan_safe && let Some(new_op) = inner_op.inverse() {
            return Some(ExprNode::Binary {
                op: new_op,
                left: left.clone(),
                right: right.clone(),
            });
        }
        if let Some(new_op) = demorgan_flip(*inner_op) {
            let not_left = ExprNode::Unary {
                op: UnaryOp::Not,
                operand: left.clone(),
            };
            let not_right = ExprNode::Unary {
                op: UnaryOp::Not,
                operand: right.clone(),
            };
            return Some(ExprNode::Binary {
                op: new_op,
                left: Box::new(not_left),
                right: Box::new(not_right),
            });
        }
    }

    None
}

/// De Morgan operator flip: `&&` ↔ `||`, otherwise `None`.
fn demorgan_flip(op: BinOp) -> Option<BinOp> {
    match op {
        BinOp::And => Some(BinOp::Or),
        BinOp::Or => Some(BinOp::And),
        _ => None,
    }
}

/// Binary-operator algebraic identities and absorbing cases.
fn reduce_binary(
    op: BinOp,
    left: &ExprNode,
    right: &ExprNode,
    bool_context: bool,
    numeric: NumericCtx<'_>,
) -> Option<ExprNode> {
    // Self-comparison tautologies for pure variable references.
    if let Some(result) = reduce_self_comparison(op, left, right, numeric) {
        return Some(result);
    }

    let lit_right = int_literal_value(right);
    let lit_left = int_literal_value(left);

    reduce_arith_identity(op, left, right, lit_left, lit_right, numeric)
        .or_else(|| reduce_pow(op, left, right, lit_right, numeric))
        .or_else(|| reduce_mod(op, left, lit_right, numeric))
        .or_else(|| reduce_shift(op, left, lit_right, numeric))
        .or_else(|| reduce_bitwise(op, left, right, lit_left, lit_right, numeric))
        .or_else(|| reduce_logical(op, left, right, lit_left, lit_right, bool_context))
}

/// `$x <cmp> $x` collapses to 0/1 when both sides are the same variable.
///
/// Only fires for pure variable references because commands or literal
/// expressions could have side effects (`[f] == [f]` might not be 1 if
/// `f` mutates state).
///
/// The *reflexive* rows (`==`, `<=`, `>=` → 1 and `!=` → 0) additionally need
/// `$x` proved non-NaN — see [`node_cannot_be_nan`].
fn reduce_self_comparison(
    op: BinOp,
    left: &ExprNode,
    right: &ExprNode,
    numeric: NumericCtx<'_>,
) -> Option<ExprNode> {
    let (ExprNode::Var { text: l, .. }, ExprNode::Var { text: r, .. }) = (left, right) else {
        return None;
    };
    if l != r {
        return None;
    }
    let k = match op {
        // Comparison operators have a string fallback in `expr` (`$x == $x`,
        // `$x < $x`, `$x eq $x` never raise), so these fold for any defined $x.
        // `$x < $x` / `$x > $x` are false for *every* value, NaN included (a
        // NaN operand makes every ordered comparison false), and string
        // identity has no NaN notion at all.
        BinOp::Lt | BinOp::Gt | BinOp::StrNe => 0,
        BinOp::StrEq => 1,
        // The reflexive folds are the ones NaN breaks: `expr {NaN == NaN}` is
        // 0, `expr {NaN <= NaN}` is 0, and `expr {NaN != NaN}` is 1 — the
        // opposite of every answer below. So they need $x proved non-NaN;
        // without the proof the expression stays as written.
        BinOp::Eq | BinOp::Le | BinOp::Ge if node_cannot_be_nan(left, numeric) => 1,
        BinOp::Ne if node_cannot_be_nan(left, numeric) => 0,
        // `$x - $x` / `$x ^ $x` fold to the *integer* literal 0, so `$x` must
        // be a provable integer, not merely numeric: for a double `$x`,
        // `expr {1.5 - 1.5}` is `0.0` (not `0`), and `^` is integer-only
        // (`expr {1.5 ^ 1.5}` errors). Folding to `0` in either case would be
        // wrong. A non-numeric `$x` still errors, which the
        // integer gate also (conservatively) declines to fold.
        BinOp::Sub | BinOp::BitXor if node_provably_integer(left, numeric) => 0,
        _ => return None,
    };
    Some(make_int_literal(k))
}

/// `+/-/*//` arithmetic identities and annihilators.
///
/// Every rewrite here removes an arithmetic operation, so the surviving
/// expression must still raise Tcl's numeric-coercion error iff the original
/// would. Identities that **keep** the operand under a double-accepting
/// operator (`x + 0`, `x - 0`, `x * 1`, `x / 1`) are gated on the operand being
/// provably *numeric*. The `x * 0 → 0` annihilator folds to an **integer**
/// literal, so it is gated on the operand being provably *integer*: for a
/// double `$x`, `expr {1.5 * 0}` is `0.0`, not `0`. Without a
/// type context both guards pass.
fn reduce_arith_identity(
    op: BinOp,
    left: &ExprNode,
    right: &ExprNode,
    lit_left: Option<i64>,
    lit_right: Option<i64>,
    numeric: NumericCtx<'_>,
) -> Option<ExprNode> {
    let num = |n: &ExprNode| node_provably_numeric(n, numeric);
    let int = |n: &ExprNode| node_provably_integer(n, numeric);
    match op {
        // x + 0 → x, 0 + x → x.
        BinOp::Add => {
            if lit_right == Some(0) && num(left) {
                return Some(left.clone());
            }
            if lit_left == Some(0) && num(right) {
                return Some(right.clone());
            }
            None
        }
        // x - 0 → x.
        BinOp::Sub if lit_right == Some(0) && num(left) => Some(left.clone()),
        // x * 1 → x, 1 * x → x, x * 0 / 0 * x → 0.
        BinOp::Mul => {
            if lit_right == Some(1) && num(left) {
                return Some(left.clone());
            }
            if lit_left == Some(1) && num(right) {
                return Some(right.clone());
            }
            // Annihilation folds to the integer literal 0 — needs integer, not
            // just numeric (a double operand yields `0.0`).
            if lit_right == Some(0) && int(left) {
                return Some(make_int_literal(0));
            }
            if lit_left == Some(0) && int(right) {
                return Some(make_int_literal(0));
            }
            None
        }
        // x / 1 → x.
        BinOp::Div if lit_right == Some(1) && num(left) => Some(left.clone()),
        _ => None,
    }
}

/// `x ** 0 → 1`, `x ** 1 → x`, `x ** 2 → x * x` for integer literal exponents.
///
/// `** 0` folds to the integer literal 1, so `x` must be provably *integer*
/// (a double base yields `1.0`). `** 1 → x` keeps `x` under an operator that
/// accepts doubles, so *numeric* suffices. `** 2 → x * x` keeps `x` on both
/// sides, preserving error semantics without a guard.
fn reduce_pow(
    op: BinOp,
    left: &ExprNode,
    _right: &ExprNode,
    lit_right: Option<i64>,
    numeric: NumericCtx<'_>,
) -> Option<ExprNode> {
    if !matches!(op, BinOp::Pow) {
        return None;
    }
    match lit_right? {
        // `x ** 0 → 1` folds to the integer literal 1 — needs integer, not just
        // numeric (`expr {1.5 ** 0}` is `1.0`). `x ** 1 → x` keeps `x`, and
        // `**` accepts doubles, so numeric suffices there.
        0 if node_provably_integer(left, numeric) => Some(make_int_literal(1)),
        1 if node_provably_numeric(left, numeric) => Some(left.clone()),
        2 => Some(ExprNode::Binary {
            op: BinOp::Mul,
            left: Box::new(left.clone()),
            right: Box::new(left.clone()),
        }),
        _ => None,
    }
}

/// `x % 1 → 0` (absorbing) and `x % pow2 → x & (pow2 - 1)`.
///
/// `%` is integer-only, so `% 1 → 0` drops `x` *and* folds to an integer: `x`
/// must be provably *integer* (`expr {1.5 % 1}` errors — folding to `0` would
/// hide it). The pow2 strength-reduction keeps `x` under `&`
/// (also integer-only), so a double `x` errors either way — error semantics
/// preserved without a guard.
fn reduce_mod(
    op: BinOp,
    left: &ExprNode,
    lit_right: Option<i64>,
    numeric: NumericCtx<'_>,
) -> Option<ExprNode> {
    if !matches!(op, BinOp::Mod) {
        return None;
    }
    let n = lit_right?;
    if n == 1 && node_provably_integer(left, numeric) {
        return Some(make_int_literal(0));
    }
    if n > 1 && (n & (n - 1)) == 0 {
        return Some(ExprNode::Binary {
            op: BinOp::BitAnd,
            left: Box::new(left.clone()),
            right: Box::new(make_int_literal(n - 1)),
        });
    }
    None
}

/// `x << 0 → x`, `x >> 0 → x`. The shift is integer-only, so `x` must be
/// provably *integer*: for a double `x`, `expr {1.5 << 0}` errors, while `$x`
/// alone returns the double — dropping the shift would hide the error.
fn reduce_shift(
    op: BinOp,
    left: &ExprNode,
    lit_right: Option<i64>,
    numeric: NumericCtx<'_>,
) -> Option<ExprNode> {
    if !matches!(op, BinOp::LShift | BinOp::RShift) {
        return None;
    }
    if lit_right == Some(0) && node_provably_integer(left, numeric) {
        Some(left.clone())
    } else {
        None
    }
}

/// Bitwise identities / annihilators: `x & 0 → 0`, `x | 0 → x`, `x ^ 0 → x`.
/// `&`/`|`/`^` are integer-only, so each rewrite (folding to `0`, or stripping
/// the operator around `x`) needs the non-literal operand provably *integer* —
/// a double operand is a Tcl error the rewrite must not hide.
fn reduce_bitwise(
    op: BinOp,
    left: &ExprNode,
    right: &ExprNode,
    lit_left: Option<i64>,
    lit_right: Option<i64>,
    numeric: NumericCtx<'_>,
) -> Option<ExprNode> {
    let int = |n: &ExprNode| node_provably_integer(n, numeric);
    match op {
        BinOp::BitAnd => {
            if lit_right == Some(0) && int(left) {
                return Some(make_int_literal(0));
            }
            if lit_left == Some(0) && int(right) {
                return Some(make_int_literal(0));
            }
            None
        }
        BinOp::BitOr | BinOp::BitXor => {
            if lit_right == Some(0) && int(left) {
                Some(left.clone())
            } else if lit_left == Some(0) && int(right) {
                Some(right.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Logical reductions for `&&` / `||` (O110).
///
/// Absorbing cases collapse to a constant boolean: `x && 0 → 0`,
/// `x || 1 → 1`. The identity cases (`x && 1`, `x || 0`) are subtler —
/// Tcl's `&&`/`||` return the normalised boolean (`0`/`1`), **not** the
/// operand value (`expr {2 && 1}` is `1`, not `2`), so the operand can be
/// returned bare only when its result is already a `0`/`1` boolean or is
/// consumed in a boolean context (where truthiness suffices). Otherwise it
/// is wrapped as `!!x` to preserve the `0`/`1` normalisation.
fn reduce_logical(
    op: BinOp,
    left: &ExprNode,
    right: &ExprNode,
    lit_left: Option<i64>,
    lit_right: Option<i64>,
    bool_context: bool,
) -> Option<ExprNode> {
    // `x && y` where y is a non-`1` literal keeps absorbing behaviour;
    // the `WordAnd`/`WordOr` spellings share the same semantics.
    match op {
        BinOp::And | BinOp::WordAnd => {
            if lit_right == Some(0) || lit_left == Some(0) {
                return Some(make_int_literal(0));
            }
            if lit_right == Some(1) {
                return Some(normalise_bool(left, bool_context));
            }
            if lit_left == Some(1) {
                return Some(normalise_bool(right, bool_context));
            }
            None
        }
        BinOp::Or | BinOp::WordOr => {
            if lit_right == Some(1) || lit_left == Some(1) {
                return Some(make_int_literal(1));
            }
            if lit_right == Some(0) {
                return Some(normalise_bool(left, bool_context));
            }
            if lit_left == Some(0) {
                return Some(normalise_bool(right, bool_context));
            }
            None
        }
        _ => None,
    }
}

/// Return `node` bare when it already yields a `0`/`1` boolean (or its
/// truthiness is all the surrounding `bool_context` needs); otherwise wrap
/// it as `!!node` so the logical operator's normalised result is preserved.
fn normalise_bool(node: &ExprNode, bool_context: bool) -> ExprNode {
    if bool_context || is_boolean_expr(node) {
        node.clone()
    } else {
        boolify(node)
    }
}

/// Wrap `node` in `!!node` to canonicalise it to a `0`/`1` result.
fn boolify(node: &ExprNode) -> ExprNode {
    use crate::expr_ast::UnaryOp;
    ExprNode::Unary {
        op: UnaryOp::Not,
        operand: Box::new(ExprNode::Unary {
            op: UnaryOp::Not,
            operand: Box::new(node.clone()),
        }),
    }
}

/// Whether `node` is known to produce a boolean (`0`/`1`) result — a
/// comparison / logical operator, a logical negation, or a boolean
/// literal.
fn is_boolean_expr(node: &ExprNode) -> bool {
    match node {
        ExprNode::Binary { op, .. } => op.yields_boolean(),
        ExprNode::Unary { op, .. } => op.yields_boolean(),
        ExprNode::Literal { text, .. } => {
            let t = text.trim();
            // Full spellings only, deliberately conservative for rewrite
            // safety — the canonical vocabulary, not a private word list.
            t == "0" || t == "1" || tcl_syntax::boolean::is_boolean_full_word(t)
        }
        _ => false,
    }
}
