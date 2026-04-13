//! Expression-level shimmer detection (S100).
//!
//! `[expr]` arithmetic operators require numeric operands; string
//! comparison operators (`eq`, `ne`, `lt`, `le`, `gt`, `ge`) require
//! string values.  Using a value of the wrong type in an expression
//! forces a silent intrep conversion:
//!
//! - A `String` variable in an arithmetic binary op (`+`, `-`, `*`, …)
//!   → `String → Int/Double` shimmer.
//! - An `Int` or `Double` variable in a string comparison op (`eq`, `ne`,
//!   `lt`, `le`, `gt`, `ge`) → `Int/Double → String` shimmer.
//!
//! This pass walks every [`Statement::AssignExpr`] and
//! [`Statement::ExprEval`] in every SCCP-executable block and recurses
//! into the expression AST looking for such mismatches.

#![allow(clippy::implicit_hasher)]

use std::collections::{HashMap, HashSet};

use tcl_lexer::Span;
use tcl_registry::TclType;

use crate::expr_ast::{BinOp, ExprNode};
use crate::ir::Statement;
use crate::naming::normalise_var_name;
use crate::ssa::{SsaFunction, ValueKey};
use crate::types::{TypeKind, TypeLattice};

use super::{type_name, ShimmerWarning};

/// Find expression-level shimmer warnings for a function.
#[must_use]
pub fn find_expr_shimmers(
    ssa: &SsaFunction,
    types: &HashMap<ValueKey, TypeLattice>,
    executable_blocks: &HashSet<String>,
) -> Vec<ShimmerWarning> {
    let mut out = Vec::new();

    for (block_name, ssa_block) in &ssa.blocks {
        if !executable_blocks.contains(block_name) {
            continue;
        }
        for ss in &ssa_block.statements {
            match &ss.statement {
                Statement::AssignExpr { expr, span, .. } => {
                    collect_expr_shimmers(expr, &ss.uses, types, *span, &mut out);
                }
                Statement::ExprEval { expr, span, .. } => {
                    collect_expr_shimmers(expr, &ss.uses, types, *span, &mut out);
                }
                _ => {}
            }
        }
    }

    out
}

fn collect_expr_shimmers(
    node: &ExprNode,
    uses: &HashMap<String, u32>,
    types: &HashMap<ValueKey, TypeLattice>,
    stmt_span: Span,
    out: &mut Vec<ShimmerWarning>,
) {
    match node {
        ExprNode::Binary { op, left, right, .. } => {
            // Recurse into children first.
            collect_expr_shimmers(left, uses, types, stmt_span, out);
            collect_expr_shimmers(right, uses, types, stmt_span, out);

            match op {
                // Arithmetic operators require numeric operands.
                BinOp::Add
                | BinOp::Sub
                | BinOp::Mul
                | BinOp::Div
                | BinOp::Mod
                | BinOp::Pow
                | BinOp::LShift
                | BinOp::RShift
                | BinOp::BitAnd
                | BinOp::BitOr
                | BinOp::BitXor => {
                    check_numeric_operand(left, uses, types, stmt_span, op, out);
                    check_numeric_operand(right, uses, types, stmt_span, op, out);
                }

                // String comparison operators: operands should be String.
                BinOp::StrEq
                | BinOp::StrNe
                | BinOp::StrLt
                | BinOp::StrLe
                | BinOp::StrGt
                | BinOp::StrGe => {
                    check_string_operand(left, uses, types, stmt_span, op, out);
                    check_string_operand(right, uses, types, stmt_span, op, out);
                }

                _ => {}
            }
        }

        ExprNode::Unary { operand, .. } => {
            collect_expr_shimmers(operand, uses, types, stmt_span, out);
        }

        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
            ..
        } => {
            collect_expr_shimmers(condition, uses, types, stmt_span, out);
            collect_expr_shimmers(true_branch, uses, types, stmt_span, out);
            collect_expr_shimmers(false_branch, uses, types, stmt_span, out);
        }

        _ => {}
    }
}

/// Emit S100 if `node` is a variable reference with a non-numeric type
/// used in a numeric arithmetic context.
fn check_numeric_operand(
    node: &ExprNode,
    uses: &HashMap<String, u32>,
    types: &HashMap<ValueKey, TypeLattice>,
    span: Span,
    op: &BinOp,
    out: &mut Vec<ShimmerWarning>,
) {
    let ExprNode::Var { name, .. } = node else {
        return;
    };
    let base = normalise_var_name(name);
    let Some(&ver) = uses.get(base) else { return };
    if ver == 0 {
        return;
    }
    let lattice = types
        .get(&(base.to_owned(), ver))
        .cloned()
        .unwrap_or_else(TypeLattice::unknown);
    if lattice.kind != TypeKind::Known {
        return;
    }
    let current = match lattice.tcl_type {
        Some(t) => t,
        None => return,
    };
    // Only flag clearly non-numeric types (String, List, Dict).
    if matches!(current, TclType::String | TclType::List | TclType::Dict) {
        out.push(ShimmerWarning {
            span,
            variable: base.to_owned(),
            from_type: current,
            to_type: TclType::Numeric,
            command: format!("expr:{op:?}"),
            in_loop: false,
            code: "S100".to_owned(),
            message: format!(
                "S100: variable '{var}' has {from} intrep used in arithmetic \
                 expression (op {op:?})",
                var = base,
                from = type_name(current),
            ),
            related: Vec::new(),
        });
    }
}

/// Emit S100 if `node` is a numeric variable used in a string comparison.
fn check_string_operand(
    node: &ExprNode,
    uses: &HashMap<String, u32>,
    types: &HashMap<ValueKey, TypeLattice>,
    span: Span,
    op: &BinOp,
    out: &mut Vec<ShimmerWarning>,
) {
    let ExprNode::Var { name, .. } = node else {
        return;
    };
    let base = normalise_var_name(name);
    let Some(&ver) = uses.get(base) else { return };
    if ver == 0 {
        return;
    }
    let lattice = types
        .get(&(base.to_owned(), ver))
        .cloned()
        .unwrap_or_else(TypeLattice::unknown);
    if lattice.kind != TypeKind::Known {
        return;
    }
    let current = match lattice.tcl_type {
        Some(t) => t,
        None => return,
    };
    // Numeric types in string comparison → shimmer to String.
    if matches!(
        current,
        TclType::Int | TclType::Double | TclType::Numeric | TclType::Boolean
    ) {
        out.push(ShimmerWarning {
            span,
            variable: base.to_owned(),
            from_type: current,
            to_type: TclType::String,
            command: format!("expr:{op:?}"),
            in_loop: false,
            code: "S100".to_owned(),
            message: format!(
                "S100: numeric variable '{var}' used in string comparison (op {op:?}); \
                 consider using == or != instead",
                var = base,
            ),
            related: Vec::new(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    /// An Int variable in arithmetic — no shimmer.
    #[test]
    fn no_expr_shimmer_int_in_arithmetic() {
        let cu = CompilationUnit::build_for(
            "set x 5\nset y [expr {$x + 1}]",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let w = find_expr_shimmers(&fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        assert!(w.is_empty(), "unexpected expr shimmers: {w:?}");
    }

    /// A String variable used in arithmetic should produce S100.
    #[test]
    fn expr_shimmer_string_in_arithmetic() {
        let cu = CompilationUnit::build_for(
            "set x \"hello\"\nset y [expr {$x + 1}]",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let w = find_expr_shimmers(&fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        let has_shimmer = w.iter().any(|sw| {
            sw.variable == "x" && sw.from_type == TclType::String
        });
        assert!(has_shimmer, "expected string-in-arithmetic shimmer, got: {w:?}");
    }

    /// An Int variable used in a string comparison should produce S100.
    ///
    /// Note: `if` conditions live in `Terminator::Branch`, not in SSA
    /// statements, so we use `set z [expr {…}]` to exercise the
    /// `AssignExpr` path that `find_expr_shimmers` covers.
    #[test]
    fn expr_shimmer_int_in_string_comparison() {
        let cu = CompilationUnit::build_for(
            "set x 42\nset z [expr {$x eq \"42\"}]",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let w = find_expr_shimmers(&fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        let has_shimmer = w
            .iter()
            .any(|sw| sw.variable == "x" && sw.from_type == TclType::Int);
        assert!(has_shimmer, "expected Int-in-string-cmp shimmer, got: {w:?}");
    }

    /// A String literal compared with `eq` — no shimmer (String is correct).
    #[test]
    fn no_expr_shimmer_string_in_string_comparison() {
        let cu = CompilationUnit::build_for(
            "set x \"hello\"\nif {$x eq \"world\"} { set y 1 }",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let w = find_expr_shimmers(&fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        // x is String; used with eq (string comparison) — no shimmer.
        let str_cmp_shimmers: Vec<_> = w
            .iter()
            .filter(|sw| sw.variable == "x" && sw.to_type == TclType::String)
            .collect();
        assert!(
            str_cmp_shimmers.is_empty(),
            "unexpected String-in-string-cmp shimmer: {str_cmp_shimmers:?}"
        );
    }
}
