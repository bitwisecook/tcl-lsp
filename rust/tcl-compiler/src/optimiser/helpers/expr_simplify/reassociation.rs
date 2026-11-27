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

//! O110 constant reassociation across `+`/`-` and `*` chains.

use super::{OperandTypes, int_literal_value, make_int_literal};
use crate::expr_ast::{BinOp, ExprNode, render_expr};
use tcl_source::limits::MAX_EXPR_NODE_DEPTH;

/// **O110 constant reassociation.** Combine the integer-literal constants
/// across a left-associative `+`/`-` (resp. `*`) chain while keeping every
/// non-constant term: `$a + 1 + 2` → `$a + 3`, `$a + 3 - 1` → `$a + 2`,
/// `$a * 2 * 3` → `$a * 6`.
///
/// Fires only when one operand is itself an additive (resp. multiplicative)
/// chain — a pure operand reorder (`1 + $a` → `$a + 1`) is suppressed as
/// noise. All non-constant terms are preserved, so numeric-coercion error
/// semantics are unchanged. The term-dropping cases that *would* need a
/// provably-numeric guard — annihilating `* 0`, dropping a lone `* 1`, and a
/// lone additive term whose constant cancels to zero (`$a + 5 - 5`) — are
/// skipped.
///
/// Regrouping changes the order the chain's terms combine in, which only
/// exact arithmetic tolerates, so every term must be a variable `types`
/// proves integer. Over a double the rounding is order-dependent: `set x
/// 10000000000000000.0; expr {$x + 1 + 2}` prints `10000000000000002.0` under
/// tclsh 8.5 to 9.1 and `expr {$x + 3}` prints `10000000000000004.0`. A
/// closed subtree (`2 + 3 + $x` → `5 + $x`) folds elsewhere and needs no
/// proof.
pub(super) fn reassociate_node(node: &ExprNode, types: &OperandTypes) -> Option<ExprNode> {
    // Every regrouped term is a variable proven integer.
    let exact = |terms: &[ExprNode]| {
        terms.iter().all(|term| {
            matches!(term, ExprNode::Var { text, .. } if types.integer.contains(text.as_str()))
        })
    };
    let ExprNode::Binary { op, left, right } = node else {
        return None;
    };
    match op {
        BinOp::Add | BinOp::Sub => {
            if !is_additive(left) && !is_additive(right) {
                return None;
            }
            let mut terms = Vec::new();
            let constant = collect_add_terms(node, &mut terms, 0)?;
            if constant == i64::MIN {
                return None; // `-constant` would overflow in the builder
            }
            if !exact(&terms) {
                return fold_closed_left(*op, left, right, collect_add_terms);
            }
            // Conservative: a lone term whose additive constant cancels to zero
            // (`$a + 5 - 5`) would emit `$a` BARE — stripping the numeric-
            // coercion error `$a` must raise when non-numeric (`expr {$a}`
            // returns the string; `expr {$a + 5 - 5}` errors). Proving `$a`
            // integer needs the SSA type lattice this AST pass can't consult,
            // so mirror the multiplicative guard (`$a * 1`) and leave it be..
            if constant == 0 && terms.len() == 1 {
                return None;
            }
            let built = build_add_expr(&terms, constant);
            (render_expr(&built) != render_expr(node)).then_some(built)
        }
        BinOp::Mul => {
            if !is_mul(left) && !is_mul(right) {
                return None;
            }
            let mut terms = Vec::new();
            let constant = collect_mul_terms(node, &mut terms, 0)?;
            // Conservative: don't drop terms without a numeric proof.
            if constant == 0 || (constant == 1 && terms.len() == 1) {
                return None;
            }
            if !exact(&terms) {
                return fold_closed_left(*op, left, right, collect_mul_terms);
            }
            let built = build_mul_expr(&terms, constant);
            (render_expr(&built) != render_expr(node)).then_some(built)
        }
        _ => None,
    }
}

/// The closed-subtree fold a chain keeps when it may not regroup: a left
/// operand made of integer literals alone evaluates to its constant before
/// anything else in the chain runs, so folding it leaves the evaluation order
/// — and a double's rounding — as written (`2 + 3 + $x` → `5 + $x`).
fn fold_closed_left(
    op: BinOp,
    left: &ExprNode,
    right: &ExprNode,
    collect: fn(&ExprNode, &mut Vec<ExprNode>, u32) -> Option<i64>,
) -> Option<ExprNode> {
    if int_literal_value(left).is_some() {
        return None;
    }
    let mut terms = Vec::new();
    let constant = collect(left, &mut terms, 0)?;
    terms.is_empty().then(|| ExprNode::Binary {
        op,
        left: Box::new(make_int_literal(constant)),
        right: Box::new(right.clone()),
    })
}

fn is_additive(n: &ExprNode) -> bool {
    matches!(
        n,
        ExprNode::Binary {
            op: BinOp::Add | BinOp::Sub,
            ..
        }
    )
}

fn is_mul(n: &ExprNode) -> bool {
    matches!(n, ExprNode::Binary { op: BinOp::Mul, .. })
}

/// Flatten an `+`/`-` chain: accumulate the literal constant and push every
/// non-literal term onto `terms`. A `-` is followed only when its RHS is an
/// integer literal (otherwise the whole node is an opaque term — a
/// non-literal subtrahend is not negated here). `None` on integer overflow.
pub(super) fn collect_add_terms(
    node: &ExprNode,
    terms: &mut Vec<ExprNode>,
    depth: u32,
) -> Option<i64> {
    // Native-stack safety net: past the cap, stop flattening and
    // treat the whole remaining subtree as one opaque term contributing the
    // additive identity — the same handling as any non-chain leaf, so the
    // reassociation stays sound (all non-constant terms preserved).
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        terms.push(node.clone());
        return Some(0);
    }
    if let ExprNode::Binary { op, left, right } = node {
        match op {
            BinOp::Add => {
                let l = collect_add_terms(left, terms, depth + 1)?;
                let r = collect_add_terms(right, terms, depth + 1)?;
                return l.checked_add(r);
            }
            BinOp::Sub => {
                if let Some(rhs) = int_literal_value(right) {
                    let l = collect_add_terms(left, terms, depth + 1)?;
                    return l.checked_sub(rhs);
                }
            }
            _ => {}
        }
    }
    if let Some(v) = int_literal_value(node) {
        return Some(v);
    }
    terms.push(node.clone());
    Some(0)
}

fn build_add_expr(terms: &[ExprNode], constant: i64) -> ExprNode {
    let Some((first, rest)) = terms.split_first() else {
        return make_int_literal(constant);
    };
    let mut result = first.clone();
    for term in rest {
        result = ExprNode::Binary {
            op: BinOp::Add,
            left: Box::new(result),
            right: Box::new(term.clone()),
        };
    }
    match constant.cmp(&0) {
        std::cmp::Ordering::Greater => ExprNode::Binary {
            op: BinOp::Add,
            left: Box::new(result),
            right: Box::new(make_int_literal(constant)),
        },
        std::cmp::Ordering::Less => ExprNode::Binary {
            op: BinOp::Sub,
            left: Box::new(result),
            right: Box::new(make_int_literal(-constant)),
        },
        std::cmp::Ordering::Equal => result,
    }
}

/// Flatten a `*` chain: multiply the literal constants, push non-literals.
/// `None` on integer overflow.
pub(super) fn collect_mul_terms(
    node: &ExprNode,
    terms: &mut Vec<ExprNode>,
    depth: u32,
) -> Option<i64> {
    // Native-stack safety net: past the cap, stop flattening and
    // treat the remaining subtree as one opaque term contributing the
    // multiplicative identity — same handling as any non-chain leaf.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        terms.push(node.clone());
        return Some(1);
    }
    if let ExprNode::Binary {
        op: BinOp::Mul,
        left,
        right,
    } = node
    {
        let l = collect_mul_terms(left, terms, depth + 1)?;
        let r = collect_mul_terms(right, terms, depth + 1)?;
        return l.checked_mul(r);
    }
    if let Some(v) = int_literal_value(node) {
        return Some(v);
    }
    terms.push(node.clone());
    Some(1)
}

fn build_mul_expr(terms: &[ExprNode], constant: i64) -> ExprNode {
    if constant == 0 {
        return make_int_literal(0);
    }
    let Some((first, rest)) = terms.split_first() else {
        return make_int_literal(constant);
    };
    let mut result = first.clone();
    for term in rest {
        result = ExprNode::Binary {
            op: BinOp::Mul,
            left: Box::new(result),
            right: Box::new(term.clone()),
        };
    }
    if constant != 1 {
        result = ExprNode::Binary {
            op: BinOp::Mul,
            left: Box::new(result),
            right: Box::new(make_int_literal(constant)),
        };
    }
    result
}
