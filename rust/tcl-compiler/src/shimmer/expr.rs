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
use tcl_core_types::DiagCode;

use tcl_lexer::Span;
use tcl_registry::TclType;

use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::expr_ast::{BinOp, ExprNode};
use crate::ir::Statement;
use crate::irules_checks::CodeFix;
use crate::naming::normalise_var_name;
use crate::sccp::cfg_order;
use crate::ssa::{SsaFunction, Symbol, ValueKey};
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
///
/// `source` is the whole compilation unit's source text — used only to build
/// the eq/ne/lt/le/gt/ge → ==/!=/</<=/>/>= quick fix (see
/// [`find_operator_fix`]); every span computed elsewhere in this pass is
/// already absolute and needs no source access.
#[must_use]
pub(crate) fn find_expr_shimmers(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    types: &HashMap<ValueKey, TypeLattice>,
    executable_blocks: &HashSet<BlockId>,
    source: &str,
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
                    let mut ctx = ExprShimmerCtx {
                        uses: &ss.uses,
                        types,
                        ssa,
                        stmt_span: *span,
                        source,
                        in_loop,
                        seen: &mut seen,
                        out: &mut out,
                    };
                    collect_expr_shimmers(&mut ctx, expr);
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
            let mut ctx = ExprShimmerCtx {
                uses: &ssa_block.exit_versions,
                types,
                ssa,
                stmt_span: branch_span,
                source,
                in_loop,
                seen: &mut seen,
                out: &mut out,
            };
            collect_expr_shimmers(&mut ctx, condition);
        }
    }

    out
}

/// Read-only context + warning sinks threaded through one expr walk.
///
/// `uses` / `stmt_span` / `source` / `in_loop` are constant for a single
/// `collect_expr_shimmers` recursion (they describe the statement whose expr
/// is being walked); `seen` / `out` accumulate de-duplicated warnings.
struct ExprShimmerCtx<'a> {
    uses: &'a HashMap<Symbol, u32>,
    types: &'a HashMap<ValueKey, TypeLattice>,
    ssa: &'a SsaFunction,
    stmt_span: Span,
    source: &'a str,
    in_loop: bool,
    seen: &'a mut HashSet<(Span, String)>,
    out: &'a mut Vec<ShimmerWarning>,
}

fn collect_expr_shimmers(ctx: &mut ExprShimmerCtx<'_>, node: &ExprNode) {
    match node {
        ExprNode::Binary {
            op, left, right, ..
        } => {
            // Recurse into children first.
            collect_expr_shimmers(ctx, left);
            collect_expr_shimmers(ctx, right);

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
                    check_numeric_operand(ctx, left, *op);
                    check_numeric_operand(ctx, right, *op);
                }

                // `==` / `!=` take the numeric-coercion path only when at least
                // one operand is provably numeric (else Tcl falls back to a
                // string compare and no shimmer occurs).
                BinOp::Eq | BinOp::Ne => {
                    if operand_looks_numeric(left, ctx.uses, ctx.types, ctx.ssa)
                        || operand_looks_numeric(right, ctx.uses, ctx.types, ctx.ssa)
                    {
                        check_numeric_operand(ctx, left, *op);
                        check_numeric_operand(ctx, right, *op);
                    }
                }

                // String comparison operators: operands should be String.
                // When *both* sides are provably numeric-safe (a numeric
                // literal, or a variable whose tracked type is numeric), the
                // rewrite to the operator's numeric equivalent
                // (eq→==, ne→!=, lt→<, le→<=, gt→>, ge→>=) is
                // semantics-preserving — Tcl's numeric comparison agrees with
                // the string comparison whenever both operands are numbers —
                // so a `CodeFix` is attached; otherwise only the (unfixable)
                // warning fires (e.g. `$n eq "abc"` — rewriting to `==` would
                // change a well-defined "always false" string compare into a
                // runtime "non-numeric string" error).
                BinOp::StrEq
                | BinOp::StrNe
                | BinOp::StrLt
                | BinOp::StrLe
                | BinOp::StrGt
                | BinOp::StrGe => {
                    let fix = (operand_looks_numeric(left, ctx.uses, ctx.types, ctx.ssa)
                        && operand_looks_numeric(right, ctx.uses, ctx.types, ctx.ssa))
                    .then(|| find_operator_fix(ctx.source, ctx.stmt_span, *op))
                    .flatten();
                    check_string_operand(ctx, left, *op, fix.clone());
                    check_string_operand(ctx, right, *op, fix);
                }

                _ => {}
            }
        }

        ExprNode::Unary { operand, .. } => {
            collect_expr_shimmers(ctx, operand);
        }

        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
            ..
        } => {
            collect_expr_shimmers(ctx, condition);
            collect_expr_shimmers(ctx, true_branch);
            collect_expr_shimmers(ctx, false_branch);
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
    uses: &HashMap<Symbol, u32>,
    types: &HashMap<ValueKey, TypeLattice>,
    ssa: &SsaFunction,
) -> bool {
    match node {
        ExprNode::Literal { .. } => true,
        ExprNode::String { text, .. } => expr_string_is_numeric(text),
        ExprNode::Var { name, .. } => {
            let base = normalise_var_name(name);
            let Some(sym) = ssa.var_symbol(base) else {
                return false;
            };
            let Some(&ver) = uses.get(&sym) else {
                return false;
            };
            if ver == 0 {
                return false;
            }
            types
                .get(&(sym, ver))
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
fn check_numeric_operand(ctx: &mut ExprShimmerCtx<'_>, node: &ExprNode, op: BinOp) {
    let ExprNode::Var { name, .. } = node else {
        return;
    };
    let base = normalise_var_name(name);
    let Some(sym) = ctx.ssa.var_symbol(base) else {
        return;
    };
    let Some(&ver) = ctx.uses.get(&sym) else {
        return;
    };
    if ver == 0 {
        return;
    }
    let lattice = ctx
        .types
        .get(&(sym, ver))
        .cloned()
        .unwrap_or_else(TypeLattice::unknown);
    if lattice.kind != TypeKind::Known {
        return;
    }
    let Some(current) = lattice.tcl_type else {
        return;
    };
    // Only flag clearly non-numeric types (String, List, Dict, ByteArray). A
    // byte array in an arithmetic context is a textbook C Tcl shimmer: Tcl
    // has no numeric intrep of its own, so `Tcl_GetNumberFromObj` falls back
    // to parsing the byte array's *string* rep (each byte relabelled as a
    // Latin-1 code point) as a number and, on success, replaces the cached
    // byte-array intrep with Int/Double — exactly the same in-place intrep
    // clobber as the String/List/Dict cases, just via the byte-array route.
    if matches!(
        current,
        TclType::String | TclType::List | TclType::Dict | TclType::ByteArray
    ) {
        // De-duplicate per (statement span, variable) within the block.
        if !ctx.seen.insert((ctx.stmt_span, base.to_owned())) {
            return;
        }
        let code = if ctx.in_loop {
            DiagCode::S101
        } else {
            DiagCode::S100
        };
        ctx.out.push(ShimmerWarning {
            span: ctx.stmt_span,
            variable: base.to_owned(),
            from_type: current,
            to_type: TclType::Numeric,
            command: format!("expr:{op:?}"),
            in_loop: ctx.in_loop,
            code,
            message: format!(
                "{code}: variable '{var}' has {from} intrep used in arithmetic \
                 expression (op {op:?})",
                var = base,
                from = type_name(current),
            ),
            related: Vec::new(),
            fixes: Vec::new(),
        });
    }
}

/// Emit a shimmer if `node` is a numeric variable used in a string
/// comparison.  S101 inside a loop body, S100 outside one.
///
/// `fix` — precomputed once per `Binary` node by the caller (identical for
/// both operands, since it targets the shared operator token) — is attached
/// only when the rewrite is provably safe; see the `BinOp::StrEq | …` match
/// arm's doc comment in [`collect_expr_shimmers`].
fn check_string_operand(
    ctx: &mut ExprShimmerCtx<'_>,
    node: &ExprNode,
    op: BinOp,
    fix: Option<CodeFix>,
) {
    let ExprNode::Var { name, .. } = node else {
        return;
    };
    let base = normalise_var_name(name);
    let Some(sym) = ctx.ssa.var_symbol(base) else {
        return;
    };
    let Some(&ver) = ctx.uses.get(&sym) else {
        return;
    };
    if ver == 0 {
        return;
    }
    let lattice = ctx
        .types
        .get(&(sym, ver))
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
        if !ctx.seen.insert((ctx.stmt_span, base.to_owned())) {
            return;
        }
        let code = if ctx.in_loop {
            DiagCode::S101
        } else {
            DiagCode::S100
        };
        let numeric_op = numeric_equivalent(op);
        let hint = if fix.is_some() {
            format!("; use '{numeric_op}' instead — both operands are numeric")
        } else {
            format!(
                "; if a numeric comparison was intended, use '{numeric_op}' \
                 instead (only safe when the other operand is provably numeric too)"
            )
        };
        ctx.out.push(ShimmerWarning {
            span: ctx.stmt_span,
            variable: base.to_owned(),
            from_type: current,
            to_type: TclType::String,
            command: format!("expr:{op:?}"),
            in_loop: ctx.in_loop,
            code,
            message: format!(
                "{code}: numeric variable '{base}' used in string comparison (op {op:?}){hint}"
            ),
            related: Vec::new(),
            fixes: fix.into_iter().collect(),
        });
    }
}

/// The numeric-comparison equivalent of a string-comparison `BinOp` (the
/// direction [`find_operator_fix`] rewrites towards). Panics on any other
/// variant — only ever called with `BinOp::Str{Eq,Ne,Lt,Le,Gt,Ge}` from
/// [`check_string_operand`]'s single call site.
fn numeric_equivalent(op: BinOp) -> &'static str {
    match op {
        BinOp::StrEq => "==",
        BinOp::StrNe => "!=",
        BinOp::StrLt => "<",
        BinOp::StrLe => "<=",
        BinOp::StrGt => ">",
        BinOp::StrGe => ">=",
        _ => unreachable!("numeric_equivalent called with a non-string-comparison BinOp"),
    }
}

/// True when `source[at..at + len]` is bounded by non-identifier characters
/// on both sides (or string edges) — i.e. `word` appears there as a whole
/// token, not as a substring of a longer identifier (`"eq"` inside
/// `"freq"`).
fn is_standalone_word_at(source: &str, at: usize, len: usize) -> bool {
    let before_ok = source[..at]
        .chars()
        .next_back()
        .is_none_or(|c| !c.is_alphanumeric() && c != '_');
    let after_ok = source[at + len..]
        .chars()
        .next()
        .is_none_or(|c| !c.is_alphanumeric() && c != '_');
    before_ok && after_ok
}

/// Locate the source text of a `BinOp::Str{Eq,Ne,Lt,Le,Gt,Ge}` operator
/// within `stmt_span`'s slice of `source` and build a [`CodeFix`] rewriting
/// it to its numeric equivalent (`numeric_equivalent`).
///
/// The operator word (`eq`, `ne`, `lt`, `le`, `gt`, `ge`) must appear
/// **exactly once**, as a standalone word (bounded by non-identifier
/// characters), within the statement's source slice — `None` otherwise
/// (source unavailable, operator not found, or more than one occurrence
/// makes the target ambiguous — e.g. `$x eq $y && $z eq $w`, two `expr`
/// operators in one statement no per-statement span can disambiguate; the
/// warning still fires, only the mechanical fix is withheld). This is a
/// deliberately conservative textual scan, not a re-parse — the same
/// "narrow, well-documented approximation" pattern already used elsewhere in
/// this codebase (e.g. `irules_checks::is_getter_form`) for fix-only, never
/// diagnosis-affecting, text lookups.
fn find_operator_fix(source: &str, stmt_span: Span, op: BinOp) -> Option<CodeFix> {
    let word = match op {
        BinOp::StrEq => "eq",
        BinOp::StrNe => "ne",
        BinOp::StrLt => "lt",
        BinOp::StrLe => "le",
        BinOp::StrGt => "gt",
        BinOp::StrGe => "ge",
        _ => return None,
    };
    let start = usize::try_from(stmt_span.start()).ok()?;
    let end = usize::try_from(stmt_span.end()).ok()?;
    let slice = source.get(start..end)?;

    let mut matches = slice
        .match_indices(word)
        .filter(|&(i, _)| is_standalone_word_at(slice, i, word.len()));
    let (offset, _) = matches.next()?;
    if matches.next().is_some() {
        return None;
    }

    let abs_start = stmt_span.start() + u32::try_from(offset).ok()?;
    let abs_end = abs_start + u32::try_from(word.len()).ok()?;
    let numeric_op = numeric_equivalent(op);
    Some(CodeFix {
        span: Span::new(abs_start, abs_end),
        new_text: numeric_op.to_owned(),
        description: format!("Use numeric comparison '{numeric_op}' instead of '{word}'"),
    })
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
        let w = find_expr_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &cu.source,
        );
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
        let w = find_expr_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &cu.source,
        );
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
        let w = find_expr_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &cu.source,
        );
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
        let w2 = find_expr_shimmers(
            &fu2.cfg,
            &fu2.ssa,
            &fu2.types,
            &fu2.sccp.executable_blocks,
            &cu2.source,
        );
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
        let w = find_expr_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &cu.source,
        );
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
        let w = find_expr_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &cu.source,
        );
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
        let w = find_expr_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &cu.source,
        );
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
        let w = find_expr_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &cu.source,
        );
        let s = w.iter().find(|sw| sw.variable == "x");
        assert!(s.is_some(), "expected expr shimmer for x in loop: {w:?}");
        let s = s.unwrap();
        assert_eq!(
            s.code,
            DiagCode::S101,
            "in-loop arithmetic shimmer must be S101"
        );
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
        let w = find_expr_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &cu.source,
        );
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

    /// A byte-array-typed variable used in arithmetic shimmers to Numeric —
    /// the same in-place intrep clobber as String/List/Dict, reached via
    /// `Tcl_GetNumberFromObj` falling back to the byte array's string rep.
    #[test]
    fn expr_shimmer_bytearray_in_arithmetic() {
        let cu = CompilationUnit::build_for(
            "set b [binary format c* {49 50}]\nset y [expr {$b + 1}]",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let w = find_expr_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &cu.source,
        );
        let has_shimmer = w
            .iter()
            .any(|sw| sw.variable == "b" && sw.from_type == TclType::ByteArray);
        assert!(
            has_shimmer,
            "expected bytearray-in-arithmetic shimmer, got: {w:?}"
        );
    }

    /// (TP) Both sides of `eq` are numeric (an Int var, an integer literal) —
    /// the rewrite to `==` is semantics-preserving, so a `CodeFix` is
    /// attached, targeting exactly the `eq` token's own span.
    #[test]
    fn expr_shimmer_eq_both_numeric_gets_fix() {
        let src = "set x 42\nset z [expr {$x eq 42}]";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::top").unwrap();
        let w = find_expr_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &cu.source,
        );
        let s = w
            .iter()
            .find(|sw| sw.variable == "x")
            .unwrap_or_else(|| panic!("expected an Int-in-string-cmp shimmer, got: {w:?}"));
        let fix = s
            .fixes
            .first()
            .unwrap_or_else(|| panic!("expected a fix for both-numeric eq, got: {s:?}"));
        assert_eq!(fix.new_text, "==");
        let op_start = src.find(" eq ").unwrap() + 1;
        assert_eq!(
            (fix.span.start(), fix.span.end()),
            (
                u32::try_from(op_start).unwrap(),
                u32::try_from(op_start + 2).unwrap()
            ),
            "fix span must cover exactly the 'eq' token: {fix:?}"
        );
    }

    /// (FP guard) `$n eq "abc"` — `abc` is not numeric, so rewriting to `==`
    /// would turn a well-defined "always false" string compare into a Tcl
    /// runtime error ("expected integer but got \"abc\""). The warning still
    /// fires (informational), but no fix is offered.
    #[test]
    fn expr_shimmer_eq_non_numeric_sibling_gets_no_fix() {
        let src = "set n 5\nset z [expr {$n eq \"abc\"}]";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::top").unwrap();
        let w = find_expr_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &cu.source,
        );
        let s = w
            .iter()
            .find(|sw| sw.variable == "n")
            .unwrap_or_else(|| panic!("expected an Int-in-string-cmp shimmer, got: {w:?}"));
        assert!(
            s.fixes.is_empty(),
            "must not offer an unsafe rewrite when the sibling isn't numeric: {s:?}"
        );
    }

    /// (TP) `le`/`ge`/etc. rewrite to their numeric equivalents too, not just
    /// `eq`/`ne`.
    #[test]
    fn expr_shimmer_le_both_numeric_gets_fix() {
        let src = "set x 3\nset y 5\nset z [expr {$x le $y}]";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::top").unwrap();
        let w = find_expr_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &cu.source,
        );
        let s = w
            .iter()
            .find(|sw| sw.variable == "x" || sw.variable == "y")
            .unwrap_or_else(|| panic!("expected an Int-in-string-cmp shimmer, got: {w:?}"));
        let fix = s
            .fixes
            .first()
            .unwrap_or_else(|| panic!("expected a fix for both-numeric le, got: {s:?}"));
        assert_eq!(fix.new_text, "<=");
    }

    /// (Ambiguity guard) Two `eq` operators in one statement — the operator
    /// word isn't unique within the statement span, so the fix is withheld
    /// (the warning itself still fires).
    #[test]
    fn expr_shimmer_ambiguous_duplicate_operator_gets_no_fix() {
        let src = "set x 1\nset y 2\nset z [expr {($x eq 1) && ($y eq 2)}]";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::top").unwrap();
        let w = find_expr_shimmers(
            &fu.cfg,
            &fu.ssa,
            &fu.types,
            &fu.sccp.executable_blocks,
            &cu.source,
        );
        let shimmers: Vec<_> = w
            .iter()
            .filter(|sw| sw.variable == "x" || sw.variable == "y")
            .collect();
        assert!(
            !shimmers.is_empty(),
            "expected Int-in-string-cmp shimmers, got: {w:?}"
        );
        assert!(
            shimmers.iter().all(|s| s.fixes.is_empty()),
            "ambiguous duplicate operator must not offer a fix: {shimmers:?}"
        );
    }
}
