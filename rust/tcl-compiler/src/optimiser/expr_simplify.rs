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
//! - **`O101`** — full constant fold on the shared expression route
//!   ([`crate::value_transfer::evaluate_expression_detached`], the one the
//!   lattice runs) on either an `ExprEval` body or an `AssignExpr`
//!   right-hand side.
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
use crate::tcl_expr_eval::{Env, format_tcl_value_with_policy};
use tcl_core_types::DiagCode;
use tcl_lexer::Span;

use super::helpers::expr_simplify::{NumericCtx, try_unwrap_expr_in_expr};
use super::{Optimisation, PassContext};

/// Run the expression-simplification pass across every function
/// in `cu`.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    use super::helpers::expr_simplify::{operand_types, operand_types_with_original_advice};
    let procedures = &cu.ir_module.procedures;
    let type_context = |fu: &crate::compilation_unit::FunctionUnit| {
        ctx.registry.map_or_else(
            || operand_types(fu),
            |registry| operand_types_with_original_advice(fu, registry),
        )
    };
    let top_numeric = type_context(&cu.top_level);
    walk_script(
        ctx,
        &cu.ir_module.top_level,
        Some(&top_numeric),
        procedures,
        0,
    );
    for (qname, proc) in &cu.ir_module.procedures {
        let numeric = cu.procedures.get(qname).map(|fu| {
            ctx.registry.map_or_else(
                || operand_types(fu),
                |registry| operand_types_with_original_advice(fu, registry),
            )
        });
        walk_script(ctx, &proc.body, numeric.as_ref(), procedures, 0);
    }
}

type Procedures = std::collections::HashMap<String, crate::ir::Procedure>;

/// `depth` is the nesting level of `script` — see
/// [`super::MAX_OPTIMISER_WALK_DEPTH`].
fn walk_script(
    ctx: &mut PassContext<'_>,
    script: &Script,
    numeric: NumericCtx<'_>,
    procedures: &Procedures,
    depth: u32,
) {
    if super::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) || !script.is_authored_source() {
        return;
    }
    for stmt in &script.statements {
        walk_statement(ctx, stmt, numeric, procedures, depth);
    }
}

fn walk_statement(
    ctx: &mut PassContext<'_>,
    stmt: &Statement,
    numeric: NumericCtx<'_>,
    procedures: &Procedures,
    depth: u32,
) {
    if matches!(
        stmt,
        Statement::Call { .. }
            | Statement::AssignValue { .. }
            | Statement::Return { expr: None, .. }
    ) {
        report_original_expression_candidates(ctx, stmt, numeric);
    }
    match stmt {
        Statement::ExprEval { span, expr, .. } => {
            try_rewrite_expr(ctx, *span, expr);
        }
        Statement::AssignExpr {
            span, name, expr, ..
        } => {
            try_rewrite_assign_expr(ctx, *span, name, expr, numeric);
        }
        // `return [expr {…}]` gets the same partial simplification as
        // `set v [expr {…}]`: `return [expr {$r ** 2}]` can become
        // `return [expr {$r * $r}]` under the same numeric proof.
        //
        // It belongs in this walker and not beside `return`'s other
        // rewrites in `propagation`, because the rewrite needs the
        // `NumericCtx` threaded here. That context is *permissive* when
        // absent — `node_provably_numeric` answers `true` for `None` — so a
        // caller without it would silently license `expr {$x + 0}` →
        // `expr {$x}`, which changes behaviour: the first raises on a
        // non-numeric `$x` and the second returns the string.
        //
        // O101 and O115 for `return` stay in `propagation`'s
        // `try_fold_return_terminator`; this walker owns O110 and O113,
        // so neither is reported twice.
        Statement::Return {
            span,
            expr: Some(expr),
            ..
        } => {
            try_rewrite_return_expr(ctx, *span, expr, numeric);
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                walk_script(ctx, &c.body, numeric, procedures, depth + 1);
            }
            if let Some(body) = else_body {
                walk_script(ctx, body, numeric, procedures, depth + 1);
            }
        }
        Statement::While { body, .. } | Statement::Catch { body, .. } => {
            walk_script(ctx, body, numeric, procedures, depth + 1);
        }
        Statement::For {
            init, next, body, ..
        } => {
            walk_script(ctx, init, numeric, procedures, depth + 1);
            walk_script(ctx, body, numeric, procedures, depth + 1);
            walk_script(ctx, next, numeric, procedures, depth + 1);
        }
        Statement::Foreach { body, .. } => walk_script(ctx, body, numeric, procedures, depth + 1),
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_script(ctx, body, numeric, procedures, depth + 1);
            for h in handlers {
                walk_script(ctx, &h.body, numeric, procedures, depth + 1);
            }
            if let Some(fb) = finally_body {
                walk_script(ctx, fb, numeric, procedures, depth + 1);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    walk_script(ctx, b, numeric, procedures, depth + 1);
                }
            }
            if let Some(db) = default_body {
                walk_script(ctx, db, numeric, procedures, depth + 1);
            }
        }
        _ => {}
    }
}

/// Conditional simplifications of retained generic invocations never carry an edit.
fn report_original_expression_candidates(
    ctx: &mut PassContext<'_>,
    stmt: &Statement,
    numeric: NumericCtx<'_>,
) {
    let Some(registry) = ctx.registry else {
        return;
    };
    let tokens = stmt.tokens();
    let config = tcl_lexer::LexerConfig::for_profile(ctx.dialect);
    let mut originals = crate::word_subst::lifted_calls(tokens, config)
        .into_iter()
        .filter_map(|call| call.tokens)
        .collect::<Vec<_>>();
    if let Some(tokens) = tokens {
        originals.push(tokens.clone());
    }
    for tokens in originals {
        let Some(advice) =
            crate::registry_invocation::original_expression_operand_advice(registry, &tokens)
        else {
            continue;
        };
        if let ExprNode::Command { text, .. } = &advice.expression
            && let Some(unwrapped) = try_unwrap_expr_in_expr(text)
            && stmt
                .tokens()
                .is_some_and(|original| original.words() == tokens.words())
            && ctx.command_mutations.trusts("expr")
            && ctx
                .fold_policy()
                .preparation_context()
                .is_some_and(|context| {
                    tokens.source_binding.as_ref().is_some_and(|binding| {
                        binding.nested_expression_normalisation(
                            registry,
                            &tokens,
                            &context,
                            Some(&advice.expression),
                        )
                    })
                })
        {
            ctx.report(Optimisation::new(
                DiagCode::O115,
                "Remove redundant nested expr",
                stmt.span(),
                format!("expr {{{unwrapped}}}"),
            ));
            continue;
        }
        let original_context = numeric
            .and_then(|types| types.original_candidate_context(advice.span))
            .unwrap_or_default();
        let rendered = crate::expr_ast::render_expr(&advice.expression);
        let (_, changed) = super::helpers::expr_simplify::instcombine_expr_typed(
            &rendered,
            false,
            Some(&original_context),
            ctx.dialect,
        );
        if changed {
            let mut candidate = Optimisation::new(
                DiagCode::O110,
                "The expression may be simplified after preserving conversions, reads and object sharing",
                advice.span,
                String::new(),
            );
            candidate.hint_only = true;
            ctx.report(candidate);
        }
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
    // All three rewrites below (full fold, instcombine, strength-reduce)
    // assume `[expr {…}]` still has builtin arithmetic semantics — a
    // `rename`/`interp alias`/redefining `proc` anywhere in the module
    // invalidates that, so decline outright.
    if !ctx.command_mutations.trusts("expr") {
        return;
    }

    // 1. Full constant fold. Only when no math-function call in the
    // expression is shadowed by a user-defined `::tcl::mathfunc::<name>`
    // proc — evaluating it would use builtin semantics that no longer
    // apply. (instcombine / strength-reduce below are pure syntactic
    // identities that don't evaluate a call's result, so they're
    // unaffected by a shadowed math function.)
    let env = Env::new();
    if let Some(val) = ctx.eval_expression_at(expr, &env, span)
        && let Some(folded) = format_tcl_value_with_policy(&val, ctx.fold_policy())
    {
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
    let (simplified, inst_changed) =
        instcombine_expr_typed(&rendered_expr, false, numeric, ctx.dialect);
    if inst_changed
        && ctx
            .expression_rewrite_equivalence_at(expr, &simplified, &Env::new(), span)
            .is_ok()
    {
        ctx.report(Optimisation::new(
            DiagCode::O110,
            "Simplify expression (instcombine)",
            full_rewrite_span(ctx.source, span),
            collapse_assign_expr_wrapper(name, &simplified),
        ));
        return;
    }
    if inst_changed
        && (report_unproved_square_candidate(ctx, expr, span)
            || report_unproved_identity_candidate(ctx, expr, span))
    {
        return;
    }
    let (reduced, sred_changed) =
        try_strength_reduce_expr_typed(&rendered_expr, numeric, ctx.dialect);
    if sred_changed
        && ctx
            .expression_rewrite_equivalence_at(expr, &reduced, &Env::new(), span)
            .is_ok()
    {
        ctx.report(Optimisation::new(
            DiagCode::O113,
            "Strength-reduce expression",
            full_rewrite_span(ctx.source, span),
            collapse_assign_expr_wrapper(name, &reduced),
        ));
    }
}

/// The O110 / O113 half of `return [expr {…}]` simplification (#1962),
/// mirroring [`try_rewrite_assign_expr`]'s second half.
///
/// The guards are deliberately the same three: a `Raw` node carries no
/// parsed form to simplify; a command substitution inside the expression
/// may have side effects a rewrite must not drop or duplicate; and `expr`
/// must be provably untouched module-wide, or `[expr {…}]` no longer has
/// builtin arithmetic semantics.
///
/// Ordering matches too — instcombine before strength reduction, because
/// the identities collapse to simpler forms while strength reduction
/// produces same-complexity rewrites — so the two statement shapes report
/// the same code for the same expression.
///
/// O101 is not attempted here: `propagation::try_fold_return_terminator`
/// already folds a constant `return [expr {…}]`, and repeating it would
/// report the same fold twice.
fn try_rewrite_return_expr(
    ctx: &mut PassContext<'_>,
    span: Span,
    expr: &ExprNode,
    numeric: NumericCtx<'_>,
) {
    use super::helpers::expr_simplify::{
        expr_has_command_subst, instcombine_expr_typed, try_strength_reduce_expr_typed,
    };
    use super::helpers::spans::full_rewrite_span;

    if matches!(expr, ExprNode::Raw { .. })
        || expr_has_command_subst(expr)
        || !ctx.command_mutations.trusts("expr")
    {
        return;
    }

    let rendered = crate::expr_ast::render_expr(expr);
    let (simplified, inst_changed) = instcombine_expr_typed(&rendered, false, numeric, ctx.dialect);
    if inst_changed
        && ctx
            .expression_rewrite_equivalence_at(expr, &simplified, &Env::new(), span)
            .is_ok()
    {
        ctx.report(Optimisation::new(
            DiagCode::O110,
            "Simplify expression (instcombine)",
            full_rewrite_span(ctx.source, span),
            collapse_return_expr_wrapper(&simplified),
        ));
        return;
    }
    if inst_changed
        && (report_unproved_square_candidate(ctx, expr, span)
            || report_unproved_identity_candidate(ctx, expr, span))
    {
        return;
    }
    let (reduced, sred_changed) = try_strength_reduce_expr_typed(&rendered, numeric, ctx.dialect);
    if sred_changed
        && ctx
            .expression_rewrite_equivalence_at(expr, &reduced, &Env::new(), span)
            .is_ok()
    {
        ctx.report(Optimisation::new(
            DiagCode::O113,
            "Strength-reduce expression",
            full_rewrite_span(ctx.source, span),
            collapse_return_expr_wrapper(&reduced),
        ));
    }
}

/// Advisory only: no replacement exists until the native operand and read
/// effects are proved. This cannot be passed to an executable rewrite consumer.
fn report_unproved_square_candidate(
    ctx: &mut PassContext<'_>,
    expression: &ExprNode,
    span: tcl_lexer::Span,
) -> bool {
    let ExprNode::Binary {
        op: crate::expr_ast::BinOp::Pow,
        left,
        right,
    } = expression
    else {
        return false;
    };
    if !matches!(left.as_ref(), ExprNode::Var { .. })
        || !matches!(right.as_ref(), ExprNode::Literal { text, .. } if text == "2")
    {
        return false;
    }
    let mut candidate = Optimisation::new(
        DiagCode::O110,
        "Squaring may use multiplication after proving native numeric operands and unchanged read, error and coercion behaviour",
        span,
        String::new(),
    );
    candidate.hint_only = true;
    ctx.report(candidate);
    true
}

/// An identity candidate does not erase an arithmetic result object or a read.
fn report_unproved_identity_candidate(
    ctx: &mut PassContext<'_>,
    expression: &ExprNode,
    span: Span,
) -> bool {
    use crate::expr_ast::BinOp;
    let ExprNode::Binary { op, left, right } = expression else {
        return false;
    };
    let identity = match (op, right.as_ref()) {
        (BinOp::Add | BinOp::Sub, ExprNode::Literal { text, .. }) => text == "0",
        (BinOp::Mul, ExprNode::Literal { text, .. }) => text == "1",
        _ => false,
    };
    matches!(left.as_ref(), ExprNode::Var { .. })
        && identity
        && ctx.report_prepared_expression_candidate(
            DiagCode::O110,
            "The arithmetic identity may be simplified after preserving conversions, reads and object sharing",
            span,
        )
}

/// Keep the native result operation after proving a partial expression rewrite.
fn collapse_return_expr_wrapper(simplified: &str) -> String {
    format!("return [expr {{{simplified}}}]")
}

/// Keep the native result operation after proving a partial expression rewrite.
fn collapse_assign_expr_wrapper(name: &str, simplified: &str) -> String {
    format!("set {name} [expr {{{simplified}}}]")
}

fn try_rewrite_expr(ctx: &mut PassContext<'_>, span: Span, expr: &ExprNode) {
    // Both rewrites below assume this statement's `expr` — and, for the
    // O115 unwrap, any nested `[expr {…}]` inside it — is the untouched
    // builtin. A `rename`/`interp alias`/redefining `proc` anywhere in the
    // module invalidates that assumption for every occurrence, so decline
    // outright rather than fold with semantics that no longer apply.
    if !ctx.command_mutations.trusts("expr") {
        return;
    }
    // O115: unwrap `[expr {…}]` in expression context. Detected
    // from the expression AST so the rewrite sees the parsed
    // form (the source span on `ExprEval` does not always cover
    // the trailing `}` of a braced body — see
    // `lowering::structured` for the token-span limitation).
    if let ExprNode::Command { text, .. } = expr
        && let Some(unwrapped) = try_unwrap_expr_in_expr(text)
        && ctx
            .ir_module
            .zip(ctx.registry)
            .is_some_and(|(module, registry)| {
                crate::math_function_binding::ExpressionMathBindings::for_module_statement(
                    module, span,
                )
                .is_some_and(|bindings| {
                    let Some(context) = ctx.fold_policy().preparation_context() else {
                        return false;
                    };
                    bindings.nested_numeric_normalisation(&context, registry, expr)
                })
            })
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
    if matches!(expr, ExprNode::Raw { .. }) {
        return;
    }
    let env = Env::new();
    if let Some(val) = ctx.eval_expression_at(expr, &env, span)
        && let Some(folded) = format_tcl_value_with_policy(&val, ctx.fold_policy())
    {
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
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap())
    }

    fn run_pass(source: &str) -> Vec<Optimisation> {
        let registry = registry();
        let cu = CompilationUnit::build_for(source, &registry, false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        ctx.dialect = registry.profile();
        ctx.registry = Some(&registry);
        ctx.ir_module = Some(&cu.ir_module);
        run(&mut ctx, &cu);
        ctx.optimisations
    }

    /// Like [`run_pass`] but populates `ctx.command_mutations` from the
    /// whole module, the way the real pipeline (`manager::optimise_unit_raw`)
    /// does — needed to exercise the `expr`-redefinition trust gate, since
    /// a bare `PassContext::new` defaults to "trusts everything".
    fn run_pass_with_mutations(source: &str) -> Vec<Optimisation> {
        let reg = registry();
        let cu = CompilationUnit::build_for(source, &reg, false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        ctx.dialect = reg.profile();
        ctx.registry = Some(&reg);
        ctx.ir_module = Some(&cu.ir_module);
        ctx.command_mutations =
            crate::command_binding::scan_module_command_mutations(&cu.ir_module, &reg);
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
    fn scoped_standalone_nested_expr_uses_original_normalisation_receipt() {
        let source = "proc f {x} {expr {[expr {$x * 2}]}}";
        assert!(run_pass_with_mutations(source).iter().any(|optimisation| {
            optimisation.code == DiagCode::O115
                && !optimisation.hint_only
                && optimisation.replacement == "expr {$x * 2}"
        }));
        for suffix in [
            "; unknown_future_entry",
            "; trace add execution expr enter callback",
            "; rename expr saved; proc expr args {return changed}",
        ] {
            assert!(
                run_pass_with_mutations(&format!("{source}{suffix}"))
                    .iter()
                    .all(|optimisation| optimisation.code != DiagCode::O115)
            );
        }
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
    fn renamed_expr_is_not_folded() {
        // FP guard: once `expr` has been renamed anywhere in the module,
        // `expr {1 + 2}` no longer means "evaluate the arithmetic
        // expression" — it means "call whatever `expr` now resolves to"
        // (here, nothing — the builtin was moved to `real_expr`). O101
        // must not fold it as if the builtin were still in place.
        let opts = run_pass_with_mutations("rename expr real_expr\nexpr {1 + 2}");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O101),
            "must not fold a renamed expr: {opts:?}",
        );
    }

    #[test]
    fn ordinary_mathfunc_call_still_folds() {
        // TN/control: no override present — abs(-5) folds as usual.
        let opts = run_pass("expr {abs(-5)}");
        assert!(
            opts.iter()
                .any(|o| o.code == DiagCode::O101 && o.replacement == "5"),
            "expected O101 fold of abs(-5), got {opts:?}",
        );
    }

    #[test]
    fn aliased_mathfunc_uses_actual_implicit_dispatch() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = registry().project_for_profile(profile);
        let source = "rename ::tcl::mathfunc::abs saved\ninterp alias {} ::tcl::mathfunc::abs {} list BOX\nexpr {abs(-3)}";
        let cu = CompilationUnit::build_for(source, &registry, false);
        let mut context = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        context.dialect = Some(profile);
        context.registry = Some(&registry);
        context.ir_module = Some(&cu.ir_module);
        context.command_mutations =
            crate::command_binding::scan_module_command_mutations(&cu.ir_module, &registry);
        run(&mut context, &cu);
        assert!(
            context
                .optimisations
                .iter()
                .all(|rewrite| rewrite.code != DiagCode::O101),
            "native implicit call returns BOX -3, not stock abs: {:?}",
            context.optimisations,
        );
    }

    #[test]
    fn native_math_alias_resolves_its_terminal_implementation_before_folding() {
        let source =
            "interp alias {} ::tcl::mathfunc::chosen {} ::tcl::mathfunc::abs; expr {chosen(-3)}";
        let rewrites = run_pass(source);
        assert!(
            rewrites
                .iter()
                .any(|rewrite| rewrite.code == DiagCode::O101 && rewrite.replacement == "3"),
            "the actual native alias call returns 3: {rewrites:?}"
        );
    }

    #[test]
    fn frozen_math_alias_prefix_bytes_do_not_prove_object_coercion_is_unobserved() {
        let source = "set prefix [list -3]; interp alias {} ::tcl::mathfunc::chosen {} ::tcl::mathfunc::abs $prefix; expr {chosen()}";
        let rewrites = run_pass(source);
        assert!(
            rewrites
                .iter()
                .all(|rewrite| rewrite.code != DiagCode::O101)
        );
    }

    #[test]
    fn later_mathfunc_replacement_does_not_poison_the_prior_native_call() {
        let source = "expr {abs(-3)}\nproc ::tcl::mathfunc::abs {x} {return LATER}\nexpr {abs(-3)}";
        let rewrites = run_pass(source);
        let folded = rewrites
            .iter()
            .filter(|rewrite| rewrite.code == DiagCode::O101)
            .collect::<Vec<_>>();
        assert_eq!(
            folded.len(),
            1,
            "only the reached stock invocation folds: {rewrites:?}"
        );
        assert_eq!(folded[0].replacement, "3");
        assert_eq!(folded[0].span.start(), 0);
    }

    #[test]
    fn namespace_mathfunc_override_is_selected_before_folding() {
        let rewrites = run_pass(
            "namespace eval N {namespace eval tcl::mathfunc {}; proc tcl::mathfunc::abs {x} {return LOCAL}; expr {abs(-3)}}",
        );
        assert!(
            rewrites
                .iter()
                .all(|rewrite| rewrite.code != DiagCode::O101),
            "actual local math handler must be preserved: {rewrites:?}"
        );
    }

    #[test]
    fn shadowed_mathfunc_call_is_not_folded() {
        // FP guard: `proc ::tcl::mathfunc::abs` shadows the builtin `abs`
        // math function everywhere in the module — O101 must not fold
        // `abs(-5)` to `5` using builtin semantics that no longer apply.
        let opts = run_pass("proc ::tcl::mathfunc::abs {x} { return 999 }\nexpr {abs(-5)}");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O101),
            "must not fold a shadowed math function: {opts:?}",
        );
    }

    #[test]
    fn shadowed_mathfunc_in_assign_expr_is_not_folded() {
        // Same guard, `set x [expr {…}]` form.
        let opts = run_pass("proc ::tcl::mathfunc::abs {x} { return 999 }\nset v [expr {abs(-5)}]");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O101),
            "must not fold a shadowed math function in AssignExpr: {opts:?}",
        );
    }

    #[test]
    fn ordinary_expr_still_folds_under_mutation_scan() {
        // TN/TP control: a module with *no* rename/alias touching `expr`
        // still folds normally even when `command_mutations` is populated
        // from a real scan (proves the gate isn't over-broad).
        let opts = run_pass_with_mutations("expr {1 + 2}");
        assert!(
            opts.iter()
                .any(|o| o.code == DiagCode::O101 && o.replacement == "3"),
            "expected O101 fold under an unrelated mutation scan, got {opts:?}",
        );
    }

    #[test]
    fn renamed_expr_nested_unwrap_is_not_rewritten() {
        // FP guard: the O115 redundant-nested-expr unwrap also assumes
        // both layers of `[expr {…}]` are builtin calls.
        let opts = run_pass_with_mutations("rename expr real_expr\nexpr {[expr {$x + 1}]}");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O115),
            "must not unwrap through a renamed expr: {opts:?}",
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

    #[test]
    fn full_expression_folds_retain_the_native_result_protocol() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let owner = tcl_registry::model::ingress::static_context_for(dialect);
            let profile =
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
            for source in [
                "set result [expr {2+2}]; puts $result",
                "proc f {} {return [expr {2+2}]}; puts [f]",
                "expr {2+2}",
            ] {
                let findings = crate::optimiser::optimise_raw_for_profile(
                    source,
                    owner.commands(),
                    Some(profile),
                );
                assert!(
                    findings
                        .iter()
                        .all(|finding| finding.code != DiagCode::O101),
                    "{dialect}: contents cannot replace native result instructions: {findings:?}"
                );
            }
        }
    }

    /// Issue #1962: the expression rewriters reached a `set` body but not a
    /// `return` one, so `return [expr {$r ** 2}]` was left alone while
    /// `set v [expr {$r ** 2}]` became `set v [expr {$r * $r}]`.
    ///
    /// Asserted as an *agreement* between the two statement shapes rather
    /// than as a fixed expectation, since that is the property the issue
    /// asks for and it cannot drift apart silently.
    #[test]
    fn return_and_set_bodies_agree_on_expression_rewrites_issue_1962() {
        let codes = |src: &str| -> Vec<String> {
            crate::optimiser::optimise_raw_for_profile(
                src,
                &tcl_registry::CommandRegistry::build_default(),
                Some(
                    tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                ),
            )
            .iter()
            .map(|o| o.code.to_string())
            .collect()
        };

        // Strength reduction, the issue's own example. Both shapes rewrite,
        // and both report the same code.
        assert_eq!(
            codes("proc square {r} {\n    return [expr {$r ** 2}]\n}\n"),
            vec!["O110".to_owned()],
            "return shape",
        );
        assert_eq!(
            codes("proc square {r} {\n    set v [expr {$r ** 2}]\n    return $v\n}\n"),
            vec!["O110".to_owned()],
            "set shape",
        );

        // The type guard is real in both, and permissive-when-absent is the
        // trap: `expr {$x + 0}` raises on a non-numeric `$x` while
        // `expr {$x}` returns the string, so dropping `+ 0` needs a proof.
        assert!(
            codes("proc id {x} {\n    return [expr {$x + 0}]\n}\n").is_empty(),
            "an unproven operand must not lose `+ 0` in a return",
        );
        assert!(
            codes("proc id {x} {\n    set v [expr {$x + 0}]\n    return $v\n}\n").is_empty(),
            "nor in a set",
        );
        // Mathematical numeric contents do not establish an already numeric
        // shared object; a pooled source string still needs its conversion.
        assert_eq!(
            codes("proc n {x} {\n    set x 4\n    return [expr {$x + 0}]\n}\n"),
            [] as [String; 0],
        );
        // Already numeric inputs permit a numeric-result operation rewrite;
        // they do not permit returning the existing operand instead of the
        // separately produced result object.
        let source = "proc n {} {set x [expr {4}]; return [expr {$x ** 2}]}";
        let findings = crate::optimiser::optimise_raw_for_profile(
            source,
            &tcl_registry::CommandRegistry::build_default(),
            Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()),
        );
        assert!(
            findings.iter().any(|finding| finding.code == DiagCode::O110
                && !finding.hint_only
                && !finding.replacement.is_empty()),
            "native numeric producer must permit the actual edit: {findings:?}"
        );
        let pooled = crate::optimiser::optimise_raw_for_profile(
            "proc n {} {set x [expr {2 + 2}]; return [expr {$x ** 2}]}",
            &tcl_registry::CommandRegistry::build_default(),
            Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()),
        );
        assert!(
            pooled
                .iter()
                .filter(|finding| finding.code == DiagCode::O110)
                .all(|finding| finding.hint_only && finding.replacement.is_empty())
        );
        assert!(
            codes("set x [expr {2 + 2}]; set result [expr {$x + 0}]; llength $result; puts [tcl::unsupported::representation $x]")
                .iter()
                .all(|code| code != "O110"),
            "numeric input proof cannot introduce result sharing observed by later conversion"
        );
        let unknown = crate::optimiser::optimise_raw_for_profile(
            "proc square {r} {return [expr {$r ** 2}]}",
            &tcl_registry::CommandRegistry::build_default(),
            Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()),
        );
        assert!(
            unknown
                .iter()
                .filter(|finding| finding.code == DiagCode::O110)
                .all(|finding| finding.hint_only && finding.replacement.is_empty())
        );

        // A command substitution may have side effects, so the rewrite must
        // not move around one.
        assert!(
            codes("proc s {x} {\n    return [expr {[llength $x] ** 2}]\n}\n").is_empty(),
            "a command substitution blocks the rewrite",
        );

        // A known constant result retains its native allocation/pool protocol.
        // The nested-wrapper diagnostic remains owned by propagation.
        assert_eq!(
            codes("proc c {} {\n    return [expr {1 + 2}]\n}\n"),
            [] as [String; 0],
            "constant contents do not license replacing the native result",
        );
        assert_eq!(
            codes("proc d {x} {\n    return [expr {[expr {$x * 2}]}]\n}\n"),
            vec!["O115".to_owned()],
            "nested unwrap stays single-reported",
        );
    }
}
