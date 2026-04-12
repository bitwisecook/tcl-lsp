//! Pattern-recognition optimiser pass (C30g, partial).
//!
//! Ported from
//! `core/compiler/optimiser/_pattern_recognition.py`. The Python
//! module has three entry points — each rewriting a common
//! high-level idiom into a more idiomatic equivalent:
//!
//! - **`optimise_incr_idioms`** (`O114`) — rewrite
//!   `set x [expr {$x ± N}]` to `incr x N` (**landed**).
//! - `optimise_string_build_chains` (`O122`) — fold
//!   accumulation chains like `set s "$s foo"` into an
//!   equivalent, cheaper form.
//! - `optimise_multi_set_packing` (`O119`) — pack a contiguous
//!   group of literal `set` commands into `lassign` / `foreach`
//!   (gated on Tcl ≤ 8.6).
//!
//! This strip lands `optimise_incr_idioms` only. The two heavier
//! rewriters are deferred — each needs token-level source
//! reconstruction that the Rust IR does not yet expose
//! (trailing-whitespace-aware command ranges for O119, plus
//! multi-statement coalescing logic).

use crate::compilation_unit::CompilationUnit;
use crate::expr_ast::{BinOp, ExprNode};
use crate::ir::{Script, Statement};
use crate::naming::normalise_var_name;

use super::{Optimisation, PassContext};

/// Run the pattern-recognition pass. Emits `O114` for every
/// `set x [expr {$x ± N}]` idiom in the module.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    walk_script(ctx, &cu.ir_module.top_level);
    for proc in cu.ir_module.procedures.values() {
        walk_script(ctx, &proc.body);
    }
}

fn walk_script(ctx: &mut PassContext<'_>, script: &Script) {
    for stmt in &script.statements {
        walk_statement(ctx, stmt);
    }
}

fn walk_statement(ctx: &mut PassContext<'_>, stmt: &Statement) {
    match stmt {
        Statement::AssignExpr { span, name, expr } => {
            if let Some(replacement) = try_incr_idiom(name, expr) {
                ctx.report(Optimisation::new(
                    "O114",
                    "Use incr instead of set/expr",
                    *span,
                    replacement,
                ));
            }
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                walk_script(ctx, &c.body);
            }
            if let Some(b) = else_body {
                walk_script(ctx, b);
            }
        }
        Statement::For {
            init, next, body, ..
        } => {
            walk_script(ctx, init);
            walk_script(ctx, next);
            walk_script(ctx, body);
        }
        Statement::While { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Foreach { body, .. } => walk_script(ctx, body),
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_script(ctx, body);
            for h in handlers {
                walk_script(ctx, &h.body);
            }
            if let Some(fb) = finally_body {
                walk_script(ctx, fb);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    walk_script(ctx, b);
                }
            }
            if let Some(b) = default_body {
                walk_script(ctx, b);
            }
        }
        _ => {}
    }
}

/// If `expr` is of the shape `$var ± literal` (where `var`
/// normalises to `target_name`), return the equivalent `incr`
/// command text. Matches Python's `_try_incr_idiom`.
///
/// Rewrites:
///
/// - `$x + 1`  → `incr x`
/// - `$x + N`  → `incr x N`        (`N` any non-zero integer)
/// - `$x - 1`  → `incr x -1`       (equivalent to `incr x -1`)
/// - `$x - N`  → `incr x -N`       (`N` a non-zero integer)
///
/// Returns `None` for anything that does not match the form.
fn try_incr_idiom(target_name: &str, expr: &ExprNode) -> Option<String> {
    let ExprNode::Binary { op, left, right } = expr else {
        return None;
    };
    match op {
        BinOp::Add => {
            let (var, lit) = extract_var_and_literal(left, right)?;
            if !var_matches(target_name, var) {
                return None;
            }
            let n = parse_int_literal(lit)?;
            Some(format_incr(target_name, n))
        }
        BinOp::Sub => {
            // Subtraction is not commutative — demand $x - N.
            let ExprNode::Var { name: var, .. } = left.as_ref() else {
                return None;
            };
            if !var_matches(target_name, var) {
                return None;
            }
            let n = parse_int_literal(right)?;
            if n == 0 {
                return None;
            }
            // `$x - N` → `incr x -N`. Use checked_neg to guard
            // against i64::MIN (whose negation overflows).
            let negated = n.checked_neg()?;
            Some(format_incr(target_name, negated))
        }
        _ => None,
    }
}

fn extract_var_and_literal<'a>(
    a: &'a ExprNode,
    b: &'a ExprNode,
) -> Option<(&'a str, &'a ExprNode)> {
    // Commutative Add: accept $var on either side.
    if let ExprNode::Var { name, .. } = a {
        return Some((name.as_str(), b));
    }
    if let ExprNode::Var { name, .. } = b {
        return Some((name.as_str(), a));
    }
    None
}

fn var_matches(target: &str, candidate: &str) -> bool {
    normalise_var_name(&format!("${candidate}")) == target
}

fn parse_int_literal(node: &ExprNode) -> Option<i64> {
    let ExprNode::Literal { text, .. } = node else {
        return None;
    };
    text.trim().parse::<i64>().ok()
}

fn format_incr(name: &str, amount: i64) -> String {
    if amount == 1 {
        format!("incr {name}")
    } else {
        format!("incr {name} {amount}")
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

    // -- helpers ------------------------------------------------------------

    #[test]
    fn var_matches_normalises_dollar_prefix() {
        assert!(var_matches("x", "x"));
        assert!(!var_matches("x", "y"));
    }

    #[test]
    fn format_incr_omits_amount_for_plus_one() {
        assert_eq!(format_incr("x", 1), "incr x");
        assert_eq!(format_incr("x", 5), "incr x 5");
        assert_eq!(format_incr("x", -3), "incr x -3");
    }

    // -- end-to-end tests ---------------------------------------------------

    #[test]
    fn set_expr_plus_one_rewrites_to_incr() {
        let opts = run_pass("set x [expr {$x + 1}]");
        let got = opts.iter().find(|o| o.code == "O114");
        assert!(got.is_some(), "expected O114, got {opts:?}");
        assert_eq!(got.unwrap().replacement, "incr x");
    }

    #[test]
    fn set_expr_plus_n_carries_the_amount() {
        let opts = run_pass("set x [expr {$x + 5}]");
        assert_eq!(
            opts.iter().find(|o| o.code == "O114").unwrap().replacement,
            "incr x 5",
        );
    }

    #[test]
    fn set_expr_minus_one_becomes_incr_negative_one() {
        let opts = run_pass("set x [expr {$x - 1}]");
        assert_eq!(
            opts.iter().find(|o| o.code == "O114").unwrap().replacement,
            "incr x -1",
        );
    }

    #[test]
    fn set_expr_other_var_is_not_incr() {
        let opts = run_pass("set x [expr {$y + 1}]");
        assert!(
            opts.iter().all(|o| o.code != "O114"),
            "different variable should not be recognised, got {opts:?}",
        );
    }

    #[test]
    fn set_expr_not_add_or_sub_is_ignored() {
        let opts = run_pass("set x [expr {$x * 2}]");
        assert!(
            opts.iter().all(|o| o.code != "O114"),
            "multiplication should not be recognised, got {opts:?}",
        );
    }

    #[test]
    fn commutative_add_accepts_literal_on_left() {
        // Tcl allows `$x + 1` and `1 + $x` — both should be
        // recognised as an incr idiom.
        let opts = run_pass("set x [expr {1 + $x}]");
        assert_eq!(
            opts.iter().find(|o| o.code == "O114").unwrap().replacement,
            "incr x",
        );
    }

    #[test]
    fn run_passes_dispatches_pattern_recognition() {
        let cu = CompilationUnit::build_for("set x [expr {$x + 1}]", &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        super::super::run_passes(&mut ctx, &cu, &[super::super::PassId::PatternRecognition]);
        assert!(
            ctx.optimisations.iter().any(|o| o.code == "O114"),
            "expected O114 via run_passes, got {:?}",
            ctx.optimisations,
        );
    }
}
