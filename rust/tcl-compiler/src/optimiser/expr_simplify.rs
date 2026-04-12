//! Expression-simplification optimiser pass (C30e).
//!
//! Walks every `ExprEval` statement in the IR (the Rust
//! lowering's representation of a bare `expr {…}` command) and
//! applies the landed [`helpers::expr_simplify`] rewriters:
//!
//! - [`super::helpers::expr_simplify::try_unwrap_expr_in_expr`]
//!   → **`O115`** (remove redundant nested `[expr {…}]`).
//! - AST-level constant folding via
//!   [`crate::tcl_expr_eval::eval_tcl_expr`] → **`O101`** (fold
//!   constant expression).
//!
//! The deeper AST-level rewriters (`InstCombine`, strength
//! reduction, strlen / `streq` simplification) are stubs until
//! their sub-strips (`C30e4` – `C30e7`) land; their diagnostic
//! codes (`O113` / `O117` / `O120` / `O110`) will start firing
//! here as soon as the stubs are replaced.
//!
//! Branch conditions (`if` / `while` / `for`) are *not* visited
//! here — those go through
//! [`super::branch_folding::optimise_branch_proc_calls`] (C30a',
//! pending) which has richer context (SSA uses, interprocedural
//! summaries). The two passes deliberately do not overlap.

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
            opts.iter().any(|o| o.code == "O101" && o.replacement == "3"),
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
