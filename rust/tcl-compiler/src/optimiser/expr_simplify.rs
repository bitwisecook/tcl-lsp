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

//! Expression-simplification optimiser pass.
//!
//! Walks `Statement::ExprEval` (a bare `expr {…}` command) and
//! `Statement::AssignExpr` (the lowered form of
//! `set name [expr {…}]`) in the IR, plus the bodies of every
//! structural-control statement (if / while / for / foreach /
//! switch / catch / try) so nested `expr` and `set … [expr …]`
//! sites are visited too.
//!
//! Diagnostics emitted by this pass:
//!
//! - **`O115`** ([`super::helpers::expr_simplify::try_unwrap_expr_in_expr`])
//!   — remove redundant nested `[expr {…}]` on a standalone
//!   `expr` statement.
//! - **`O101`** — full constant fold via
//!   [`crate::tcl_expr_eval::eval_tcl_expr`] on either an
//!   `ExprEval` body or an `AssignExpr` right-hand side.
//! - **`O110`** ([`super::helpers::expr_simplify::instcombine_expr`])
//!   — instcombine identities (`x + 0` → `x`, etc.) on an
//!   `AssignExpr` right-hand side.  Skipped on `AssignExpr`
//!   bodies that contain a command substitution.
//! - **`O113`** ([`super::helpers::expr_simplify::try_strength_reduce_expr`])
//!   — strength-reduction (`x*1` → `x`, `x**2` → `x*x`,
//!   `x % 2^k` → `x & (2^k-1)`) on an `AssignExpr` right-hand
//!   side.
//!
//! The other AST-level rewriters
//! ([`super::helpers::expr_simplify::try_strlen_simplify_expr`]
//! `O117`,
//! [`super::helpers::expr_simplify::try_eq_ne_string_compare_simplify_expr`]
//! `O120`) fire through the
//! `branch_folding::propagate_into_branches` cascade
//! that has the richer context (SSA uses, interprocedural
//! summaries) those rewrites need to be sound on branch
//! conditions.  The two passes deliberately do not overlap on
//! `if` / `while` / `for` conditions — those go through
//! `branch_folding::optimise_branch_proc_calls` only.

use crate::compilation_unit::CompilationUnit;
use crate::expr_ast::ExprNode;
use crate::ir::{Script, Statement};
use crate::tcl_expr_eval::{Env, eval_tcl_expr, format_tcl_value};
use tcl_core_types::DiagCode;
use tcl_lexer::Span;

use super::helpers::expr_simplify::{NumericCtx, try_unwrap_expr_in_expr};
use super::{Optimisation, PassContext};

/// Run the expression-simplification pass across every function
/// in `cu`.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    use super::helpers::expr_simplify::numeric_var_names;
    let top_numeric = numeric_var_names(&cu.top_level);
    walk_script(
        ctx,
        &cu.ir_module.top_level,
        ctx.dialect,
        Some(&top_numeric),
    );
    for (qname, proc) in &cu.ir_module.procedures {
        let numeric = cu.procedures.get(qname).map(numeric_var_names);
        walk_script(ctx, &proc.body, ctx.dialect, numeric.as_ref());
    }
}

fn walk_script(
    ctx: &mut PassContext<'_>,
    script: &Script,
    dialect: Option<&str>,
    numeric: NumericCtx<'_>,
) {
    for stmt in &script.statements {
        walk_statement(ctx, stmt, dialect, numeric);
    }
    let _ = dialect;
}

fn walk_statement(
    ctx: &mut PassContext<'_>,
    stmt: &Statement,
    dialect: Option<&str>,
    numeric: NumericCtx<'_>,
) {
    match stmt {
        Statement::ExprEval { span, expr } => {
            try_rewrite_expr(ctx, *span, expr);
        }
        Statement::AssignExpr {
            span, name, expr, ..
        } => {
            try_rewrite_assign_expr(ctx, *span, name, expr, numeric);
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                walk_script(ctx, &c.body, dialect, numeric);
            }
            if let Some(body) = else_body {
                walk_script(ctx, body, dialect, numeric);
            }
        }
        Statement::While { body, .. } | Statement::Catch { body, .. } => {
            walk_script(ctx, body, dialect, numeric);
        }
        Statement::For {
            init, next, body, ..
        } => {
            walk_script(ctx, init, dialect, numeric);
            walk_script(ctx, body, dialect, numeric);
            walk_script(ctx, next, dialect, numeric);
        }
        Statement::Foreach { body, .. } => walk_script(ctx, body, dialect, numeric),
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_script(ctx, body, dialect, numeric);
            for h in handlers {
                walk_script(ctx, &h.body, dialect, numeric);
            }
            if let Some(fb) = finally_body {
                walk_script(ctx, fb, dialect, numeric);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    walk_script(ctx, b, dialect, numeric);
                }
            }
            if let Some(db) = default_body {
                walk_script(ctx, db, dialect, numeric);
            }
        }
        _ => {}
    }
}

/// Fold `set name [expr {…}]` via the standard chain:
///
/// 1. Full constant fold (`expr {2 + 3}` → `5`) → O101
/// 2. Strength reduction (`$x * 1` → `$x`, `$x ** 2` → `$x * $x`,
///    `$x % 8` → `$x & 7`) → O113
/// 3. `InstCombine` identities (`$x + 0` → `$x`, `$x * 0` → `0`)
///    → O110
///
/// Applies to `set name [expr {…}]` assignments. Skipped when the
/// expression contains a command substitution (side-effect risk).
fn try_rewrite_assign_expr(
    ctx: &mut PassContext<'_>,
    span: Span,
    name: &str,
    expr: &ExprNode,
    numeric: NumericCtx<'_>,
) {
    use super::helpers::expr_simplify::{
        expr_has_command_subst, instcombine_expr_typed, try_strength_reduce_expr_typed,
    };
    use super::helpers::spans::full_rewrite_span;

    if matches!(expr, ExprNode::Raw { .. }) {
        return;
    }
    // Skip expressions containing command substitutions — those
    // could have side effects that must not be lost.
    if expr_has_command_subst(expr) {
        return;
    }

    // 1. Full constant fold.
    let env = Env::new();
    if let Some(val) = eval_tcl_expr(expr, &env) {
        let folded = format_tcl_value(val);
        let original = crate::expr_ast::render_expr(expr);
        if folded != original.trim() {
            // Safe-word check: the folded value must inline as a
            // bare argument to `set`. Numbers and safe identifiers
            // qualify; strings with Tcl metacharacters don't.
            let needs_quoting = folded.is_empty()
                || folded.contains([
                    ' ', '\t', '\n', '\r', '$', '[', ']', '{', '}', '"', '\\', '\0', ';',
                ]);
            if !needs_quoting {
                ctx.report(Optimisation::new(
                    DiagCode::O101,
                    "Fold constant expression",
                    full_rewrite_span(ctx.source, span),
                    format!("set {name} {folded}"),
                ));
                return;
            }
        }
    }

    // 2. + 3. Partial simplification via instcombine / strength
    // reduction. The helpers operate on text form, so render
    // first, then re-wrap in ``expr { … }``.
    //
    // Priority: InstCombine (identities and reassociation) fires
    // before strength-reduction, because the identities collapse to
    // simpler forms (`$x + 0` → `$x`) while strength-reduction
    // produces same-complexity rewrites (`$x ** 2` → `$x * $x`).
    // Running instcombine first keeps the categorisation stable.
    let rendered_expr = crate::expr_ast::render_expr(expr);
    let (simplified, inst_changed) = instcombine_expr_typed(&rendered_expr, false, numeric);
    if inst_changed {
        ctx.report(Optimisation::new(
            DiagCode::O110,
            "Simplify expression (instcombine)",
            full_rewrite_span(ctx.source, span),
            collapse_assign_expr_wrapper(name, &simplified),
        ));
        return;
    }
    let (reduced, sred_changed) = try_strength_reduce_expr_typed(&rendered_expr, numeric);
    if sred_changed {
        ctx.report(Optimisation::new(
            DiagCode::O113,
            "Strength-reduce expression",
            full_rewrite_span(ctx.source, span),
            collapse_assign_expr_wrapper(name, &reduced),
        ));
    }
}

/// Build the replacement for ``set name [expr {…}]`` after a
/// simplifier produced `simplified`. When `simplified` is a
/// safe-to-inline integer literal (or bare identifier), emit the
/// unwrapped ``set name SIMPLIFIED`` form so cascading passes
/// don't have to re-fold the trivial ``expr {K}``. Otherwise
/// preserve the ``expr { … }`` wrapper.
fn collapse_assign_expr_wrapper(name: &str, simplified: &str) -> String {
    let trimmed = simplified.trim();
    let looks_literal = !trimmed.is_empty()
        && !trimmed.contains([
            ' ', '\t', '\n', '\r', '$', '[', ']', '{', '}', '"', '\\', '\0', ';',
        ]);
    if looks_literal {
        return format!("set {name} {trimmed}");
    }
    format!("set {name} [expr {{{simplified}}}]")
}

fn try_rewrite_expr(ctx: &mut PassContext<'_>, span: Span, expr: &ExprNode) {
    // O115: unwrap `[expr {…}]` in expression context. Detected
    // from the expression AST so the rewrite sees the parsed
    // form (the source span on `ExprEval` does not always cover
    // the trailing `}` of a braced body — see
    // `lowering::structured` for the token-span limitation).
    if let ExprNode::Command { text, .. } = expr
        && let Some(unwrapped) = try_unwrap_expr_in_expr(text)
    {
        ctx.report(Optimisation::new(
            DiagCode::O115,
            "Remove redundant nested expr",
            span,
            format!("expr {{{unwrapped}}}"),
        ));
        return;
    }

    // O101: fold a fully constant expression. Only report when
    // the rewrite would actually change the source text — an
    // expression like `expr {42}` folds to itself and a no-op
    // quick-fix is misleading.
    let env = Env::new();
    if matches!(expr, ExprNode::Raw { .. }) {
        return;
    }
    if let Some(val) = eval_tcl_expr(expr, &env) {
        let folded = format_tcl_value(val);
        // Compare against the original body text slice when it is
        // recoverable; the outer span covers the whole `expr …`
        // command so we look at the `ExprNode::Command`-free
        // fallback — render the parsed expression back to text and
        // use that as the baseline.
        let original = crate::expr_ast::render_expr(expr);
        if folded == original.trim() {
            return;
        }
        ctx.report(Optimisation::new(
            DiagCode::O101,
            "Fold constant expression",
            span,
            folded,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::CommandRegistry;

    use crate::interprocedural::InterproceduralAnalysis;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    fn run_pass(source: &str) -> Vec<Optimisation> {
        let cu = CompilationUnit::build_for(source, &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        run(&mut ctx, &cu);
        ctx.optimisations
    }

    #[test]
    fn constant_expr_folds_to_literal() {
        let opts = run_pass("expr {1 + 2}");
        assert!(
            opts.iter()
                .any(|o| o.code == DiagCode::O101 && o.replacement == "3"),
            "expected O101 fold, got {opts:?}",
        );
    }

    #[test]
    fn nested_expr_unwrap() {
        // `expr {[expr {$x + 1}]}` — the outer expr body is
        // `[expr {$x + 1}]`, which is a redundant wrapper.
        let opts = run_pass("expr {[expr {$x + 1}]}");
        assert!(
            opts.iter()
                .any(|o| o.code == DiagCode::O115 && o.replacement.contains("$x + 1")),
            "expected O115 unwrap, got {opts:?}",
        );
    }

    #[test]
    fn variable_expression_produces_nothing() {
        let opts = run_pass("expr {$x + 1}");
        assert!(
            opts.iter()
                .all(|o| o.code != DiagCode::O101 && o.code != DiagCode::O115),
            "unexpected rewrite: {opts:?}",
        );
    }

    #[test]
    fn run_passes_dispatches_expr_simplify() {
        let cu = CompilationUnit::build_for("expr {1 + 2}", &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        super::super::run_passes(&mut ctx, &cu, &[super::super::PassId::ExprSimplify]);
        assert!(
            ctx.optimisations.iter().any(|o| o.code == DiagCode::O101),
            "expected O101 via run_passes, got {:?}",
            ctx.optimisations,
        );
    }
}
