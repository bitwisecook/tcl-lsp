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

use std::collections::{HashMap, HashSet};

use tcl_lexer::Span;
use tcl_registry::TclType;

use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::expr_ast::{BinOp, ExprNode};
use crate::ir::Statement;
use crate::naming::normalise_var_name;
use crate::sccp::cfg_order;
use crate::ssa::{SsaFunction, ValueKey};
use crate::types::{TypeKind, TypeLattice};

use super::graph::loop_body_blocks;
use super::{ShimmerWarning, type_name};

/// Find expression-level shimmer warnings for a function.
///
/// Covers two expression sites:
/// 1. **`AssignExpr` / `ExprEval` statements** — `set x [expr {…}]` and
///    standalone `expr {…}`.
/// 2. **`Terminator::Branch` conditions** — the predicate of every
///    `if`/`while`/`for` construct.  Variable versions are resolved from
///    the block's `exit_versions` map (the versions live at the end of
///    the block, which is when the condition is evaluated).
#[must_use]
pub(crate) fn find_expr_shimmers(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    types: &HashMap<ValueKey, TypeLattice>,
    executable_blocks: &HashSet<BlockId>,
) -> Vec<ShimmerWarning> {
    let mut out = Vec::new();
    let loop_blocks = loop_body_blocks(cfg);

    for block_id in cfg_order(cfg) {
        if !executable_blocks.contains(&block_id) {
            continue;
        }
        let Some(ssa_block) = ssa.blocks.get(&block_id) else {
            continue;
        };
        // An expr-operator shimmer inside a loop body re-converts the operand
        // every iteration (S101); outside a loop it is one-time (S100).
        let in_loop = loop_blocks.contains(cfg.block_name(block_id));
        // Per-block de-duplication keyed on (statement span, variable): several
        // operands of the same statement that name the same variable emit one
        // warning, not one per operand.
        let mut seen: HashSet<(Span, String)> = HashSet::new();

        // 1. SSA statements: AssignExpr and ExprEval.
        for ss in &ssa_block.statements {
            match &ss.statement {
                Statement::AssignExpr { expr, span, .. }
                | Statement::ExprEval { expr, span, .. } => {
                    collect_expr_shimmers(
                        expr, &ss.uses, types, *span, in_loop, &mut seen, &mut out,
                    );
                }
                _ => {}
            }
        }

        // 2. Branch terminator condition (if/while/for predicate).
        if let Some(block) = cfg.blocks.get(&block_id)
            && let Some(Terminator::Branch {
                condition, span, ..
            }) = &block.terminator
        {
            let branch_span = span.unwrap_or_else(|| Span::new(0, 0));
            // Use exit_versions: those are the variable versions in scope
            // when the condition is evaluated.
            collect_expr_shimmers(
                condition,
                &ssa_block.exit_versions,
                types,
                branch_span,
                in_loop,
                &mut seen,
                &mut out,
            );
        }
    }

    out
}

fn collect_expr_shimmers(
    node: &ExprNode,
    uses: &HashMap<String, u32>,
    types: &HashMap<ValueKey, TypeLattice>,
    stmt_span: Span,
    in_loop: bool,
    seen: &mut HashSet<(Span, String)>,
    out: &mut Vec<ShimmerWarning>,
) {
    match node {
        ExprNode::Binary {
            op, left, right, ..
        } => {
            // Recurse into children first.
            collect_expr_shimmers(left, uses, types, stmt_span, in_loop, seen, out);
            collect_expr_shimmers(right, uses, types, stmt_span, in_loop, seen, out);

            match op {
                // Arithmetic, bitwise, logical, and *ordering* comparison
                // operators are always a numeric context (Tcl `<`/`<=`/`>`/`>=`
                // compare numerically when possible).
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
                | BinOp::BitXor
                | BinOp::And
                | BinOp::Or
                | BinOp::Lt
                | BinOp::Le
                | BinOp::Gt
                | BinOp::Ge => {
                    check_numeric_operand(left, uses, types, stmt_span, *op, in_loop, seen, out);
                    check_numeric_operand(right, uses, types, stmt_span, *op, in_loop, seen, out);
                }

                // `==` / `!=` take the numeric-coercion path only when at least
                // one operand is provably numeric (else Tcl falls back to a
                // string compare and no shimmer occurs).
                BinOp::Eq | BinOp::Ne => {
                    if operand_looks_numeric(left, uses, types)
                        || operand_looks_numeric(right, uses, types)
                    {
                        check_numeric_operand(
                            left, uses, types, stmt_span, *op, in_loop, seen, out,
                        );
                        check_numeric_operand(
                            right, uses, types, stmt_span, *op, in_loop, seen, out,
                        );
                    }
                }

                // String comparison operators: operands should be String.
                BinOp::StrEq
                | BinOp::StrNe
                | BinOp::StrLt
                | BinOp::StrLe
                | BinOp::StrGt
                | BinOp::StrGe => {
                    check_string_operand(left, uses, types, stmt_span, *op, in_loop, seen, out);
                    check_string_operand(right, uses, types, stmt_span, *op, in_loop, seen, out);
                }

                _ => {}
            }
        }

        ExprNode::Unary { operand, .. } => {
            collect_expr_shimmers(operand, uses, types, stmt_span, in_loop, seen, out);
        }

        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
            ..
        } => {
            collect_expr_shimmers(condition, uses, types, stmt_span, in_loop, seen, out);
            collect_expr_shimmers(true_branch, uses, types, stmt_span, in_loop, seen, out);
            collect_expr_shimmers(false_branch, uses, types, stmt_span, in_loop, seen, out);
        }

        _ => {}
    }
}

/// True when `node` is provably numeric-looking — gates the conditional
/// `==` / `!=` numeric-shimmer check.  The SCCP-CONST arm is omitted;
/// the literal / numeric-string / typed-var arms cover the shimmer
/// cases the syntactic types reach.
fn operand_looks_numeric(
    node: &ExprNode,
    uses: &HashMap<String, u32>,
    types: &HashMap<ValueKey, TypeLattice>,
) -> bool {
    match node {
        ExprNode::Literal { .. } => true,
        ExprNode::String { text, .. } => expr_string_is_numeric(text),
        ExprNode::Var { name, .. } => {
            let base = normalise_var_name(name);
            let Some(&ver) = uses.get(base) else {
                return false;
            };
            if ver == 0 {
                return false;
            }
            types
                .get(&(base.to_owned(), ver))
                .filter(|l| l.kind == TypeKind::Known)
                .and_then(|l| l.tcl_type)
                .is_some_and(|t| {
                    matches!(
                        t,
                        TclType::Int | TclType::Double | TclType::Numeric | TclType::Boolean
                    )
                })
        }
        _ => false,
    }
}

/// True when `text` (an `ExprNode::String` body or raw value, possibly still
/// wrapped in `{}` / `"`) parses as an int, float, or Tcl boolean literal.
fn expr_string_is_numeric(text: &str) -> bool {
    let mut s = text.trim();
    if s.len() >= 2
        && ((s.starts_with('{') && s.ends_with('}')) || (s.starts_with('"') && s.ends_with('"')))
    {
        s = &s[1..s.len() - 1];
    }
    let s = s.trim();
    if s.is_empty() {
        return false;
    }
    if s.parse::<i64>().is_ok() || s.parse::<f64>().is_ok() {
        return true;
    }
    matches!(
        s.to_ascii_lowercase().as_str(),
        "true" | "false" | "yes" | "no" | "on" | "off"
    )
}

/// Emit a shimmer if `node` is a variable reference with a non-numeric
/// type used in a numeric arithmetic context.  The code is S101 inside a
/// loop body (per-iteration conversion) and S100 outside one.
#[allow(clippy::too_many_arguments)]
fn check_numeric_operand(
    node: &ExprNode,
    uses: &HashMap<String, u32>,
    types: &HashMap<ValueKey, TypeLattice>,
    span: Span,
    op: BinOp,
    in_loop: bool,
    seen: &mut HashSet<(Span, String)>,
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
    let Some(current) = lattice.tcl_type else {
        return;
    };
    // Only flag clearly non-numeric types (String, List, Dict).
    if matches!(current, TclType::String | TclType::List | TclType::Dict) {
        // De-duplicate per (statement span, variable) within the block.
        if !seen.insert((span, base.to_owned())) {
            return;
        }
        let code = if in_loop { "S101" } else { "S100" };
        out.push(ShimmerWarning {
            span,
            variable: base.to_owned(),
            from_type: current,
            to_type: TclType::Numeric,
            command: format!("expr:{op:?}"),
            in_loop,
            code: code.to_owned(),
            message: format!(
                "{code}: variable '{var}' has {from} intrep used in arithmetic \
                 expression (op {op:?})",
                var = base,
                from = type_name(current),
            ),
            related: Vec::new(),
        });
    }
}

/// Emit a shimmer if `node` is a numeric variable used in a string
/// comparison.  S101 inside a loop body, S100 outside one.
#[allow(clippy::too_many_arguments)]
fn check_string_operand(
    node: &ExprNode,
    uses: &HashMap<String, u32>,
    types: &HashMap<ValueKey, TypeLattice>,
    span: Span,
    op: BinOp,
    in_loop: bool,
    seen: &mut HashSet<(Span, String)>,
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
    let Some(current) = lattice.tcl_type else {
        return;
    };
    // Numeric types in string comparison → shimmer to String.
    if matches!(
        current,
        TclType::Int | TclType::Double | TclType::Numeric | TclType::Boolean
    ) {
        // De-duplicate per (statement span, variable) within the block.
        if !seen.insert((span, base.to_owned())) {
            return;
        }
        let code = if in_loop { "S101" } else { "S100" };
        out.push(ShimmerWarning {
            span,
            variable: base.to_owned(),
            from_type: current,
            to_type: TclType::String,
            command: format!("expr:{op:?}"),
            in_loop,
            code: code.to_owned(),
            message: format!(
                "{code}: numeric variable '{base}' used in string comparison (op {op:?}); \
                 consider using == or != instead"
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
        let cu = CompilationUnit::build_for("set x 5\nset y [expr {$x + 1}]", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let w = find_expr_shimmers(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
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
        let w = find_expr_shimmers(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        let has_shimmer = w
            .iter()
            .any(|sw| sw.variable == "x" && sw.from_type == TclType::String);
        assert!(
            has_shimmer,
            "expected string-in-arithmetic shimmer, got: {w:?}"
        );
    }

    /// A String variable compared with `==` against a numeric literal takes
    /// the numeric-coercion path and shimmers; comparing against a non-numeric
    /// string stays on the string path and does not.
    #[test]
    fn expr_shimmer_string_eq_numeric_literal() {
        let cu = CompilationUnit::build_for(
            "set s [string trim hello]\nset y [expr {$s == \"5\"}]",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let w = find_expr_shimmers(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        assert!(
            w.iter().any(|sw| sw.variable == "s"),
            "string == numeric literal must shimmer: {w:?}"
        );
        // `$s == "hello"` — both string, no numeric coercion, no shimmer.
        let cu2 = CompilationUnit::build_for(
            "set s [string trim hello]\nset y [expr {$s == \"hello\"}]",
            &registry(),
            false,
        );
        let fu2 = cu2.function("::top").unwrap();
        let w2 = find_expr_shimmers(&fu2.cfg, &fu2.ssa, &fu2.types, &fu2.sccp.executable_blocks);
        assert!(
            !w2.iter().any(|sw| sw.variable == "s"),
            "string == string must not shimmer: {w2:?}"
        );
    }

    /// An Int variable used in a string comparison inside an `AssignExpr` produces S100.
    #[test]
    fn expr_shimmer_int_in_string_comparison_assign_expr() {
        let cu =
            CompilationUnit::build_for("set x 42\nset z [expr {$x eq \"42\"}]", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let w = find_expr_shimmers(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        let has_shimmer = w
            .iter()
            .any(|sw| sw.variable == "x" && sw.from_type == TclType::Int);
        assert!(
            has_shimmer,
            "expected Int-in-string-cmp shimmer, got: {w:?}"
        );
    }

    /// An Int variable used in a string comparison inside an `if` condition
    /// (`Terminator::Branch`) produces S100 via the branch-condition path.
    #[test]
    fn expr_shimmer_int_in_if_branch_condition() {
        let cu = CompilationUnit::build_for(
            "set x 42\nif {$x eq \"42\"} { set y 1 }",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let w = find_expr_shimmers(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        let has_shimmer = w
            .iter()
            .any(|sw| sw.variable == "x" && sw.from_type == TclType::Int);
        assert!(
            has_shimmer,
            "expected Int-in-string-cmp shimmer in if-condition, got: {w:?}"
        );
    }

    /// The same variable used in several operands of one expression emits a
    /// single shimmer, not one per operand (per-block `(span, var)` dedup).
    #[test]
    fn expr_shimmer_dedups_repeated_operand() {
        let cu = CompilationUnit::build_for(
            "set x \"hi\"\nset y [expr {$x + $x + $x}]",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let w = find_expr_shimmers(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        let xs: Vec<_> = w.iter().filter(|sw| sw.variable == "x").collect();
        assert_eq!(
            xs.len(),
            1,
            "repeated operand must emit one shimmer, got: {xs:?}"
        );
    }

    /// An arithmetic shimmer inside a loop body is a per-iteration cost
    /// (S101); the same shimmer outside a loop is one-time (S100).
    #[test]
    fn expr_shimmer_in_loop_is_s101() {
        let cu = CompilationUnit::build_for(
            "proc f {l} {\n  foreach x $l {\n    set y [expr {$x + 1}]\n  }\n}\n",
            &registry(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        let w = find_expr_shimmers(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
        let s = w.iter().find(|sw| sw.variable == "x");
        assert!(s.is_some(), "expected expr shimmer for x in loop: {w:?}");
        let s = s.unwrap();
        assert_eq!(s.code, "S101", "in-loop arithmetic shimmer must be S101");
        assert!(s.in_loop);
    }

    /// A String literal compared with `eq` — no shimmer (String is correct type).
    #[test]
    fn no_expr_shimmer_string_in_string_comparison() {
        let cu = CompilationUnit::build_for(
            "set x \"hello\"\nif {$x eq \"world\"} { set y 1 }",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let w = find_expr_shimmers(&fu.cfg, &fu.ssa, &fu.types, &fu.sccp.executable_blocks);
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
