//! Expression-simplification optimiser pass (C30e + follow-up
//! C30e4–C30e7).
//!
//! Walks every `ExprEval` statement in the IR (the Rust
//! lowering's representation of a bare `expr {…}` command) and
//! applies the rewriters in [`super::helpers::expr_simplify`]:
//!
//! - [`super::helpers::expr_simplify::try_unwrap_expr_in_expr`]
//!   → **`O115`** (remove redundant nested `[expr {…}]`).
//! - AST-level constant folding via
//!   [`crate::tcl_expr_eval::eval_tcl_expr`] → **`O101`** (fold
//!   constant expression).
//!
//! The deeper AST-level rewriters (`instcombine_expr`,
//! `try_strength_reduce_expr`, `try_strlen_simplify_expr`,
//! `try_eq_ne_string_compare_simplify_expr`) all landed under
//! sub-strips C30e4–C30e7. Their diagnostic codes (`O113` /
//! `O117` / `O120` / `O110`) fire through the
//! [`super::branch_folding::propagate_into_branches`] cascade,
//! which has the richer context (SSA uses, interprocedural
//! summaries) those rewriters need to be sound — the standalone
//! `expr` statement form would re-fire diagnostics already
//! emitted by the propagation cascade, so this pass deliberately
//! sticks to the O115 / O101 pair for `Statement::ExprEval`.
//!
//! Branch conditions (`if` / `while` / `for`) go through
//! [`super::branch_folding::optimise_branch_proc_calls`] which
//! has richer context (SSA uses, interprocedural summaries).
//! The two passes deliberately do not overlap.

use crate::compilation_unit::CompilationUnit;
use crate::expr_ast::ExprNode;
use crate::ir::{Script, Statement};
use crate::tcl_expr_eval::{eval_tcl_expr, format_tcl_value, Env};
use tcl_lexer::Span;

use super::helpers::expr_simplify::try_unwrap_expr_in_expr;
use super::{Optimisation, PassContext};

/// Run the expression-simplification pass across every function
/// in `cu`.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    walk_script(ctx, &cu.ir_module.top_level, ctx.dialect);
    for proc in cu.ir_module.procedures.values() {
        walk_script(ctx, &proc.body, ctx.dialect);
    }
}

fn walk_script(ctx: &mut PassContext<'_>, script: &Script, dialect: Option<&str>) {
    for stmt in &script.statements {
        walk_statement(ctx, stmt, dialect);
    }
    let _ = dialect;
}

fn walk_statement(ctx: &mut PassContext<'_>, stmt: &Statement, dialect: Option<&str>) {
    match stmt {
        Statement::ExprEval { span, expr } => {
            try_rewrite_expr(ctx, *span, expr);
        }
        Statement::AssignExpr { span, name, expr } => {
            try_rewrite_assign_expr(ctx, *span, name, expr);
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                walk_script(ctx, &c.body, dialect);
            }
            if let Some(body) = else_body {
                walk_script(ctx, body, dialect);
            }
        }
        Statement::While { body, .. } | Statement::Catch { body, .. } => {
            walk_script(ctx, body, dialect);
        }
        Statement::For {
            init, next, body, ..
        } => {
            walk_script(ctx, init, dialect);
            walk_script(ctx, body, dialect);
            walk_script(ctx, next, dialect);
        }
        Statement::Foreach { body, .. } => walk_script(ctx, body, dialect),
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_script(ctx, body, dialect);
            for h in handlers {
                walk_script(ctx, &h.body, dialect);
            }
            if let Some(fb) = finally_body {
                walk_script(ctx, fb, dialect);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    walk_script(ctx, b, dialect);
                }
            }
            if let Some(db) = default_body {
                walk_script(ctx, db, dialect);
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
/// Matches the Python `_expr_simplify` behaviour for
/// `IRAssignExpr` nodes. Skipped when the expression contains a
/// command substitution (side-effect risk).
fn try_rewrite_assign_expr(ctx: &mut PassContext<'_>, span: Span, name: &str, expr: &ExprNode) {
    use super::helpers::expr_simplify::{
        expr_has_command_subst, instcombine_expr, try_strength_reduce_expr,
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
                    "O101",
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
    // Match the Python priority: InstCombine (identities and
    // reassociation) fires before strength-reduction, because the
    // identities collapse to simpler forms (`$x + 0` → `$x`)
    // while strength-reduction produces same-complexity rewrites
    // (`$x ** 2` → `$x * $x`). Running instcombine first keeps
    // the categorisation in line with the Python tests.
    let rendered_expr = crate::expr_ast::render_expr(expr);
    let (simplified, inst_changed) = instcombine_expr(&rendered_expr, false);
    if inst_changed {
        ctx.report(Optimisation::new(
            "O110",
            "Simplify expression (instcombine)",
            full_rewrite_span(ctx.source, span),
            collapse_assign_expr_wrapper(name, &simplified),
        ));
        return;
    }
    let (reduced, sred_changed) = try_strength_reduce_expr(&rendered_expr);
    if sred_changed {
        ctx.report(Optimisation::new(
            "O113",
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
    if let ExprNode::Command { text, .. } = expr {
        if let Some(unwrapped) = try_unwrap_expr_in_expr(text) {
            ctx.report(Optimisation::new(
                "O115",
                "Remove redundant nested expr",
                span,
                format!("expr {{{unwrapped}}}"),
            ));
            return;
        }
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
            "O101",
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
                .any(|o| o.code == "O101" && o.replacement == "3"),
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
                .any(|o| o.code == "O115" && o.replacement.contains("$x + 1")),
            "expected O115 unwrap, got {opts:?}",
        );
    }

    #[test]
    fn variable_expression_produces_nothing() {
        let opts = run_pass("expr {$x + 1}");
        assert!(
            opts.iter().all(|o| o.code != "O101" && o.code != "O115"),
            "unexpected rewrite: {opts:?}",
        );
    }

    #[test]
    fn run_passes_dispatches_expr_simplify() {
        let cu = CompilationUnit::build_for("expr {1 + 2}", &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        super::super::run_passes(&mut ctx, &cu, &[super::super::PassId::ExprSimplify]);
        assert!(
            ctx.optimisations.iter().any(|o| o.code == "O101"),
            "expected O101 via run_passes, got {:?}",
            ctx.optimisations,
        );
    }
}
