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

//! Expression-simplification helpers.
//!
//! A toolkit of AST-level expression rewriters consumed by the
//! propagation, branch-folding, and pattern-recognition passes:
//!
//! - [`try_fold_expr`] — constant-fold an expression text on the shared
//!   expression route, the one the lattice runs
//!   ([`crate::value_transfer::evaluate_expression_detached`]).
//! - [`try_unwrap_expr_in_expr`] — unwrap a redundant
//!   `[expr {…}]` in expression context (`O115`).
//! - [`substitute_expr_constants`] — replace `$var` references
//!   with their SCCP-proved literals.
//! - [`instcombine_expr`] — bottom-up fixpoint of local
//!   instcombine rewrites (capped at 16 iterations).
//! - [`try_strength_reduce_expr`] — `x+0` / `x-0` / `x*0` /
//!   `x*1` / `x/1` / `x**2` / `x % 2^k` strength-reduction set.
//! - [`try_strlen_simplify_expr`] — `[string length OP] == 0`
//!   → `OP eq ""`.
//! - [`try_eq_ne_string_compare_simplify_expr`] — `==`/`!=`
//!   against an `ExprNode::String` literal → `eq`/`ne`.
//!
//! The four AST rewriters are wired into
//! `branch_folding::propagate_into_branches`:
//! `substitute_expr_constants` runs first to build a working
//! text, then the AST rewriters are probed in priority order —
//! `strength_reduce` → `strlen` → `streq` → `instcombine`.  The
//! first rewriter that changes text wins its diagnostic code
//! (`O113` / `O117` / `O120` / `O110`); `O100` ("Propagate
//! constants into branch expression") only fires when
//! substitution changed the text *and* none of the AST
//! rewriters did.

use std::collections::HashSet;

use crate::compilation_unit::FunctionUnit;
use crate::expr_ast::{BinOp, ExprNode, ExprOffset, render_expr};
#[cfg(test)]
use crate::expr_parser::parse_expr_for_profile;
#[cfg(test)]
use crate::sccp::BuiltinFoldInputs;
use crate::types::{TclType, TypeKind, TypeLattice};
use tcl_source::limits::MAX_EXPR_NODE_DEPTH;

mod constant_folding;
mod numeric_proofs;
mod reassociation;
mod strength_reduction;
mod string_comparisons;
mod structural_equivalence;

#[cfg(test)]
use constant_folding::is_numeric_literal;
pub use constant_folding::{
    SubstitutionResult, substitute_expr_constants, substitute_expr_constants_for_execution,
    substitute_expr_constants_in_context, try_fold_expr, try_fold_expr_with_constants,
    try_unwrap_expr_in_expr,
};
#[cfg(test)]
use numeric_proofs::is_nan_string_in_any_release;
#[cfg(test)]
use numeric_proofs::{
    is_numeric_string_in_any_release, is_numeric_string_in_every_release, is_numeric_string_under,
    node_cannot_be_nan,
};
use reassociation::reassociate_node;
#[cfg(test)]
use reassociation::{collect_add_terms, collect_mul_terms};
use strength_reduction::strength_reduce_node;
#[cfg(test)]
pub use strength_reduction::try_strength_reduce_expr;
pub use strength_reduction::try_strength_reduce_expr_typed;
pub(crate) use string_comparisons::eq_ne_compares_as_strings;
use string_comparisons::streq_promote_node;
pub use string_comparisons::{try_eq_ne_string_compare_simplify_expr, try_strlen_simplify_expr};
pub use structural_equivalence::{
    structural_condition_equivalence, structural_rewrite_equivalence,
};

/// Operand type facts for the current function: which variable names are
/// provably *numeric* (Int / Double / Numeric / Boolean) and which are provably
/// *integer* (Int). The integer set is a subset of the numeric set.
///
/// The two are distinct because they gate different rewrites. An identity that
/// keeps its operand under an operator that also accepts doubles (`$x + 0`,
/// `$x * 1`) only needs the operand *numeric*. But an identity that either
/// folds to an **integer literal** by dropping an operand (`$x * 0 → 0`,
/// `$x ** 0 → 1`, `$x - $x → 0`) or removes an **integer-only** operator
/// (`<<`, `>>`, `&`, `|`, `^`, `%`) needs the operand *integer*: for a
/// double operand Tcl yields a double (`1.5 * 0` → `0.0`, not `0`) or raises
/// "can't use floating-point value as operand" — either way the integer-result
/// / operator-drop rewrite would be wrong.
#[derive(Debug, Default, Clone)]
pub struct OperandTypes {
    numeric: HashSet<String>,
    integer: HashSet<String>,
    original_advice: std::collections::HashMap<tcl_lexer::Span, (HashSet<String>, HashSet<String>)>,
}

#[cfg(test)]
impl OperandTypes {
    /// A context proving `names` numeric but *not* integer (Double-typed).
    fn numeric_only(names: &[&str]) -> Self {
        Self {
            numeric: names.iter().map(|s| format!("${s}")).collect(),
            integer: HashSet::new(),
            ..Self::default()
        }
    }

    /// A context proving `names` integer (and hence numeric).
    fn integer(names: &[&str]) -> Self {
        let set: HashSet<String> = names.iter().map(|s| format!("${s}")).collect();
        Self {
            numeric: set.clone(),
            integer: set,
            ..Self::default()
        }
    }
}

/// A type context for the current function, or `None` when no type lattice is
/// available. Passed to the `*_typed` entry points so the operand-dropping
/// identities fire only on provably typed operands. `None` keeps the
/// unguarded, aggressive behaviour for callers (and tests) that have no type
/// lattice.
pub type NumericCtx<'a> = Option<&'a OperandTypes>;

/// Build conditional type consensus for original expression operands. Every
/// occurrence with the same written reference must have an exact retained read
/// and agree on the required type. These facts only select rewrite candidates;
/// the native execution-equivalence gate independently licenses an edit.
#[must_use]
pub fn operand_types(fu: &FunctionUnit) -> OperandTypes {
    use std::collections::HashMap;
    let mut operands: HashMap<String, (bool, bool)> = HashMap::new();
    for (&block, body) in &fu.cfg.blocks {
        for (index, statement) in body.statements.iter().enumerate() {
            let (expression, base) = match statement {
                crate::ir::Statement::AssignExpr {
                    expr, expr_base, ..
                }
                | crate::ir::Statement::ExprEval {
                    expr, expr_base, ..
                }
                | crate::ir::Statement::Return {
                    expr: Some(expr),
                    expr_base,
                    ..
                } => (expr, *expr_base),
                _ => continue,
            };
            record_operand_types(
                expression,
                base,
                crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index),
                fu,
                &mut operands,
            );
        }
        let (expression, base) = match body.terminator.as_ref() {
            Some(crate::cfg::Terminator::Branch {
                condition,
                condition_base,
                ..
            }) => (condition, *condition_base),
            Some(crate::cfg::Terminator::Return {
                expr: Some(expr),
                expr_base,
                ..
            }) => (expr, *expr_base),
            _ => continue,
        };
        record_operand_types(
            expression,
            base,
            crate::ssa::SsaSourceView::at_terminator(&fu.ssa, block),
            fu,
            &mut operands,
        );
    }
    let mut out = OperandTypes::default();
    for (text, (numeric, integer)) in operands {
        if numeric {
            out.numeric.insert(text.clone());
        }
        if integer {
            out.integer.insert(text);
        }
    }
    out
}

fn record_operand_types(
    expression: &ExprNode,
    base: Option<u32>,
    view: crate::ssa::SsaSourceView<'_>,
    fu: &FunctionUnit,
    operands: &mut std::collections::HashMap<String, (bool, bool)>,
) {
    for node in expression.variable_nodes() {
        let ExprNode::Var { text, .. } = node else {
            continue;
        };
        let lattice = view
            .read_expression_variable(node, base)
            .and_then(|read| fu.types.get(&(read.symbol, read.version?)));
        let numeric = lattice.is_some_and(lattice_is_numeric);
        let integer = lattice.is_some_and(lattice_is_integer);
        operands
            .entry(text.clone())
            .and_modify(|value| {
                value.0 &= numeric;
                value.1 &= integer;
            })
            .or_insert((numeric, integer));
    }
}

/// Retain separately scoped candidate types for original expression children.
/// The exact site map is consumed only by no-edit advice; normal rewriters keep
/// the represented-read type sets from `operand_types`.
pub fn operand_types_with_original_advice(
    fu: &FunctionUnit,
    registry: &tcl_registry::CommandRegistry,
) -> OperandTypes {
    let mut out = operand_types(fu);
    for (&block, body) in &fu.ssa.blocks {
        for index in 0..body.statements.len() {
            let Some(tokens) =
                crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index).source_tokens()
            else {
                continue;
            };
            let Some(binding) = tokens.source_binding.as_ref() else {
                continue;
            };
            let Some(reads) = binding.declaration_expression_reads(registry, tokens) else {
                continue;
            };
            let Some(dialect) = binding
                .declaration_operand_layout_advice(tokens)
                .map(|advice| advice.dialect())
            else {
                continue;
            };
            let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
            let mut calls = crate::word_subst::lifted_calls(Some(tokens), config);
            let Some(child) = calls.pop().and_then(|call| call.tokens) else {
                continue;
            };
            let Some(advice) =
                crate::registry_invocation::original_expression_operand_advice(registry, &child)
            else {
                continue;
            };
            let mut typed = std::collections::HashMap::<String, (bool, bool)>::new();
            for read in reads {
                let lattice = read
                    .diagnostic_version(&fu.ssa, block, index, registry)
                    .and_then(|key| fu.types.get(&key));
                let numeric = lattice.is_some_and(lattice_is_numeric);
                let integer = lattice.is_some_and(lattice_is_integer);
                typed
                    .entry(read.spelling().to_owned())
                    .and_modify(|facts| {
                        facts.0 &= numeric;
                        facts.1 &= integer;
                    })
                    .or_insert((numeric, integer));
            }
            let mut numeric = HashSet::new();
            let mut integer = HashSet::new();
            for (spelling, facts) in typed {
                if facts.0 {
                    numeric.insert(spelling.clone());
                }
                if facts.1 {
                    integer.insert(spelling);
                }
            }
            out.original_advice.insert(advice.span, (numeric, integer));
        }
    }
    out
}

impl OperandTypes {
    pub(crate) fn original_candidate_context(&self, site: tcl_lexer::Span) -> Option<Self> {
        let (numeric, integer) = self.original_advice.get(&site)?;
        Some(Self {
            numeric: numeric.clone(),
            integer: integer.clone(),
            ..Self::default()
        })
    }
}

/// Whether a type-lattice element is a known numeric Tcl type.
fn lattice_is_numeric(t: &TypeLattice) -> bool {
    t.kind() == TypeKind::Known
        && matches!(
            t.tcl_type(),
            Some(TclType::Int | TclType::Double | TclType::Numeric | TclType::Boolean)
        )
}

/// Whether a type-lattice element is a known *integer* Tcl type. `Double` and
/// the catch-all `Numeric` (which may be a double) are excluded; `Boolean` is
/// excluded too, since a boolean-typed operand may hold the word `true`, which
/// is not an integer in `expr` (`expr {true << 0}` errors).
fn lattice_is_integer(t: &TypeLattice) -> bool {
    t.kind() == TypeKind::Known && matches!(t.tcl_type(), Some(TclType::Int))
}

// instcombine / strength-reduce / strlen / streq

/// InstCombine-style fixpoint simplification of an
/// expression text.
///
/// Parses `expr`, runs the AST simplifier until fixpoint, then
/// renders the result back to text. Returns `(new_text, changed)`
/// where `changed` indicates whether the rendered output differs
/// from `expr.trim()`. Unparseable inputs and expressions
/// containing command substitutions are returned unchanged.
///
/// The fixpoint composes all the simplifiers —
/// strength-reduction folds, strlen canonicalisation, eq/ne
/// promotion, and DeMorgan-shaped reductions — into a single
/// pass. Callers that need only a specific transform should use
/// the narrow helpers instead.
#[must_use]
#[cfg(test)]
pub fn instcombine_expr(expr: &str, bool_context: bool) -> (String, bool) {
    instcombine_expr_typed(expr, bool_context, None, None)
}

/// As [`instcombine_expr`], but with a numeric-type context so the
/// operand-dropping identities (`$x + 0` → `$x`, `$x * 0` → `0`, …) fire
/// only when the dropped operand is provably numeric. See [`NumericCtx`].
#[must_use]
pub fn instcombine_expr_typed(
    expr: &str,
    bool_context: bool,
    numeric: NumericCtx<'_>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> (String, bool) {
    let trimmed = expr.trim();
    let parsed = crate::expr_parser::parse_expr_for_profile(trimmed, dialect);
    if matches!(parsed, ExprNode::Raw { .. }) || expr_has_command_subst(&parsed) {
        return (expr.to_owned(), false);
    }
    let simplified = simplify_to_fixpoint(&parsed, bool_context, numeric);
    let rendered = render_expr(&simplified);
    // Suppress O110 noise. When the canonical re-render differs from the
    // input only in whitespace (e.g. `$x<0` → `$x < 0`), it is a spacing
    // preference, not a real finding — report no change. Structural
    // rewrites (paren removal, identity folds, operand reordering, …) all
    // alter non-whitespace characters and still register as changed.
    let changed = strip_ws(&rendered) != strip_ws(trimmed);
    (rendered, changed)
}

/// Return `expr` with all whitespace removed — the whitespace-insensitive
/// comparison key for the O110 noise guard.
fn strip_ws(expr: &str) -> String {
    expr.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Apply one pass of local simplifications to `node`, returning
/// the rewritten subtree. Used as the step function in
/// [`simplify_to_fixpoint`].
fn simplify_node_once(
    node: &ExprNode,
    bool_context: bool,
    numeric: NumericCtx<'_>,
    depth: u32,
) -> ExprNode {
    use crate::expr_ast::UnaryOp;
    // Native-stack safety net: this bottom-up rewriter recurses
    // once per `ExprNode` level. Past the cap, pass the node through
    // unchanged (no rewrite) rather than recurse — a safe no-op for a
    // simplifier, and the same shape it returns for any node it can't rewrite.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return node.clone();
    }
    // First, recurse into children — bottom-up rewriting. The boolean
    // context propagates only where the operand's *value* is consumed as a
    // truth value: the operands of `&&`/`||`/`!` and a ternary condition.
    // Comparison/arithmetic operands consume the operand's full value, so
    // they reset the context to `false`.
    let lowered = match node {
        ExprNode::Binary { op, left, right } => {
            let child_bool = matches!(op, BinOp::And | BinOp::Or | BinOp::WordAnd | BinOp::WordOr);
            ExprNode::Binary {
                op: *op,
                left: Box::new(simplify_node_once(left, child_bool, numeric, depth + 1)),
                right: Box::new(simplify_node_once(right, child_bool, numeric, depth + 1)),
            }
        }
        ExprNode::Unary { op, operand } => {
            let child_bool = matches!(op, UnaryOp::Not | UnaryOp::WordNot);
            ExprNode::Unary {
                op: *op,
                operand: Box::new(simplify_node_once(operand, child_bool, numeric, depth + 1)),
            }
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => ExprNode::Ternary {
            condition: Box::new(simplify_node_once(condition, true, numeric, depth + 1)),
            true_branch: Box::new(simplify_node_once(
                true_branch,
                bool_context,
                numeric,
                depth + 1,
            )),
            false_branch: Box::new(simplify_node_once(
                false_branch,
                bool_context,
                numeric,
                depth + 1,
            )),
        },
        other => other.clone(),
    };

    // Apply local rewrites at this level in priority order.
    if let Some(rewritten) = strength_reduce_node(&lowered, bool_context, numeric) {
        return rewritten;
    }
    if let Some(rewritten) = streq_promote_node(&lowered) {
        return rewritten;
    }
    let untyped = OperandTypes::default();
    if let Some(rewritten) = reassociate_node(&lowered, numeric.unwrap_or(&untyped)) {
        return rewritten;
    }
    lowered
}

/// Run [`simplify_node_once`] until the AST stops changing.
fn simplify_to_fixpoint(node: &ExprNode, bool_context: bool, numeric: NumericCtx<'_>) -> ExprNode {
    let mut cur = node.clone();
    for _ in 0..16 {
        let next = simplify_node_once(&cur, bool_context, numeric, 0);
        if render_expr(&next) == render_expr(&cur) {
            return next;
        }
        cur = next;
    }
    cur
}

/// Extract an integer literal from an [`ExprNode::Literal`],
/// ignoring anything else.
fn int_literal_value(node: &ExprNode) -> Option<i64> {
    let ExprNode::Literal { text, .. } = node else {
        return None;
    };
    text.trim().parse::<i64>().ok()
}

fn make_int_literal(value: i64) -> ExprNode {
    ExprNode::Literal {
        text: value.to_string(),
        start: 0 as ExprOffset,
        end: 0 as ExprOffset,
    }
}

/// Return `true` when `node` contains any command-substitution
/// subtree.
#[must_use]
pub fn expr_has_command_subst(node: &ExprNode) -> bool {
    // Public entry: the top of an expression tree is nesting depth 0; the
    // recursion cap lives in [`expr_has_command_subst_at`].
    expr_has_command_subst_at(node, 0)
}

fn expr_has_command_subst_at(node: &ExprNode, depth: u32) -> bool {
    // Native-stack safety net: past the cap, assume "yes, has a
    // command substitution" — the conservative direction, since callers use
    // this to *suppress* an optimisation when a command sub is present, so a
    // false `true` only forgoes a rewrite, never enables an unsound one.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return true;
    }
    match node {
        ExprNode::Command { .. } => true,
        ExprNode::Binary { left, right, .. } => {
            expr_has_command_subst_at(left, depth + 1)
                || expr_has_command_subst_at(right, depth + 1)
        }
        ExprNode::Unary { operand, .. } => expr_has_command_subst_at(operand, depth + 1),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            expr_has_command_subst_at(condition, depth + 1)
                || expr_has_command_subst_at(true_branch, depth + 1)
                || expr_has_command_subst_at(false_branch, depth + 1)
        }
        ExprNode::Call { args, .. } => args.iter().any(|a| expr_has_command_subst_at(a, depth + 1)),
        // A `"…"` operand substitutes, so a `[cmd]` in it runs: O110 rewrote
        // `expr {"[incr x]" && 0}` to `0`, losing the `incr` (#2227, found in
        // review).
        ExprNode::String { text, .. } => crate::word_subst::quoted_operand_body(text).is_some(),
        ExprNode::Literal { .. }
        | ExprNode::Var { .. }
        | ExprNode::Raw { .. }
        // A word, not an expression: it holds no command substitution.
        | ExprNode::CompiledWord { .. } => false,
    }
}

#[cfg(test)]
mod tests;
