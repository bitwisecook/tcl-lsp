//! Pattern-recognition optimiser pass (C30g).
//!
//! Ported from
//! `core/compiler/optimiser/_pattern_recognition.py`. Entry points:
//!
//! - **`optimise_incr_idioms`** (`O114`) — rewrite
//!   `set x [expr {$x ± N}]` to `incr x N`.
//! - **`optimise_end_offset_indexes`** (`O128`) — rewrite length
//!   arithmetic index args to `end` / `end-N` (in [`super::end_offset`]).
//! - **`optimise_string_build_chains`** (`O104` / `O130`) — fold
//!   write-only `set`+`append`/`lappend` build chains into a single
//!   `set` (in [`super::chain_fold`]).
//! - **`optimise_multi_set_packing`** (`O119`) — detect three
//!   or more contiguous `set` commands with safe-literal
//!   values and emit a hint-only `O119` suggesting a
//!   `lassign {lit1 lit2 …} var1 var2 …` replacement
//!   (appropriate on Tcl 8.5 / 8.6; Tcl 9.0 prefers individual
//!   sets).
//!
//! O119 remains hint-only. O104/O130 now emit applied folds; the source
//! span allocation matches the canonical
//! `docs/generated/optimisation_codes.md` table.

use std::collections::HashSet;

use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::expr_ast::{BinOp, ExprNode};
use crate::ir::{Script, Statement};
use crate::naming::normalise_var_name;
use crate::types::{TclType, TypeKind, TypeLattice};

use super::helpers::spans::full_rewrite_span;
use super::{Optimisation, PassContext};

/// Run the pattern-recognition pass.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    let top_ints = int_var_names(&cu.top_level);
    walk_script(ctx, &cu.ir_module.top_level, &top_ints);
    for (qname, proc) in &cu.ir_module.procedures {
        let ints = cu.procedures.get(qname).map(int_var_names).unwrap_or_default();
        walk_script(ctx, &proc.body, &ints);
    }
    // O128 — end-offset index rewrites (its own segment-level walk over
    // the same source).
    super::end_offset::run(ctx, cu);
    // O104 / O130 — applied write-only build-chain folds (replaces the old
    // hint-only detector; owns its own per-function escape gate).
    super::chain_fold::run(ctx, cu);
}

/// Names whose **every** SSA version is a known `TclType::Int`. A name absent
/// here is treated as not provably integer, so the `set/expr → incr` rewrite
/// (O114) is suppressed — matching Python, which proves the loop variable is
/// `INT` (not `DOUBLE` / `NUMERIC` / `BOOLEAN`) at the use point before
/// rewriting. `expr {$x + 1}` silently promotes a float operand, whereas
/// `incr` errors, so the function-level join (all versions must be `INT`) is a
/// sound over-approximation of the per-use check.
fn int_var_names(fu: &FunctionUnit) -> HashSet<String> {
    use std::collections::HashMap;
    let mut acc: HashMap<&str, bool> = HashMap::new();
    for ((name, _ver), lattice) in &fu.types {
        let is_int = lattice_is_int(lattice);
        acc.entry(name.as_str())
            .and_modify(|v| *v = *v && is_int)
            .or_insert(is_int);
    }
    acc.into_iter()
        .filter(|(_, ok)| *ok)
        .map(|(n, _)| n.to_owned())
        .collect()
}

/// Whether a type-lattice element is a known `TclType::Int`.
fn lattice_is_int(t: &TypeLattice) -> bool {
    t.kind == TypeKind::Known && t.tcl_type == Some(TclType::Int)
}

fn walk_script(ctx: &mut PassContext<'_>, script: &Script, int_vars: &HashSet<String>) {
    detect_multi_set_packing(ctx, script);
    for stmt in &script.statements {
        walk_statement(ctx, stmt, int_vars);
    }
}

/// O119: find three or more consecutive `set VAR LITERAL`
/// statements with bare-literal values; emit a hint-only
/// rewrite suggestion.
fn detect_multi_set_packing(ctx: &mut PassContext<'_>, script: &Script) {
    let mut i = 0;
    while i < script.statements.len() {
        let start = i;
        let mut vars: Vec<String> = Vec::new();
        while i < script.statements.len() {
            let Statement::AssignConst { name, value, .. } = &script.statements[i] else {
                break;
            };
            if value.contains(['$', '[', '\\', ' ', '\n', '\r']) {
                break;
            }
            vars.push(name.clone());
            i += 1;
        }
        if vars.len() >= 3 {
            // Span the entire run.
            let span_start = script.statements[start].span().start();
            let span_end = script.statements[i - 1].span().end();
            let span = tcl_lexer::Span::new(span_start, span_end);
            let mut opt = Optimisation::new(
                "O119",
                format!(
                    "Pack {} consecutive literal sets into `lassign`",
                    vars.len()
                ),
                span,
                "",
            );
            opt.hint_only = true;
            ctx.report(opt);
        }
        if i == start {
            i += 1;
        }
    }
}


fn walk_statement(ctx: &mut PassContext<'_>, stmt: &Statement, int_vars: &HashSet<String>) {
    match stmt {
        Statement::AssignExpr { span, name, expr } => {
            if let Some(replacement) = try_incr_idiom(name, expr, int_vars) {
                ctx.report(Optimisation::new(
                    "O114",
                    "Use incr instead of set/expr",
                    full_rewrite_span(ctx.source, *span),
                    replacement,
                ));
            }
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                walk_script(ctx, &c.body, int_vars);
            }
            if let Some(b) = else_body {
                walk_script(ctx, b, int_vars);
            }
        }
        Statement::For {
            init, next, body, ..
        } => {
            walk_script(ctx, init, int_vars);
            walk_script(ctx, next, int_vars);
            walk_script(ctx, body, int_vars);
        }
        Statement::While { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Foreach { body, .. } => walk_script(ctx, body, int_vars),
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_script(ctx, body, int_vars);
            for h in handlers {
                walk_script(ctx, &h.body, int_vars);
            }
            if let Some(fb) = finally_body {
                walk_script(ctx, fb, int_vars);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    walk_script(ctx, b, int_vars);
                }
            }
            if let Some(b) = default_body {
                walk_script(ctx, b, int_vars);
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
///
/// D5-O114 soundness gate: `target_name` must be provably `TclType::Int`
/// (present in `int_vars`). `expr {$x + 1}` silently promotes a float
/// operand (`1.5` → `2.5`), whereas `incr x` errors with *expected integer
/// but got "1.5"* — so without an integer proof the rewrite is unsound.
fn try_incr_idiom(
    target_name: &str,
    expr: &ExprNode,
    int_vars: &HashSet<String>,
) -> Option<String> {
    if !int_vars.contains(target_name) {
        return None;
    }
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
        // The seeding `set x 0` makes `x` provably INT, satisfying the
        // O114 soundness gate.
        let opts = run_pass("set x 0\nset x [expr {$x + 1}]");
        let got = opts.iter().find(|o| o.code == "O114");
        assert!(got.is_some(), "expected O114, got {opts:?}");
        assert_eq!(got.unwrap().replacement, "incr x");
    }

    #[test]
    fn set_expr_plus_n_carries_the_amount() {
        let opts = run_pass("set x 0\nset x [expr {$x + 5}]");
        assert_eq!(
            opts.iter().find(|o| o.code == "O114").unwrap().replacement,
            "incr x 5",
        );
    }

    #[test]
    fn set_expr_minus_one_becomes_incr_negative_one() {
        let opts = run_pass("set x 5\nset x [expr {$x - 1}]");
        assert_eq!(
            opts.iter().find(|o| o.code == "O114").unwrap().replacement,
            "incr x -1",
        );
    }

    #[test]
    fn set_expr_on_float_var_is_not_incr() {
        // D5-O114: `x` is DOUBLE here, so `expr {$x + 1}` promotes to a
        // float while `incr` would error — the rewrite must not fire.
        let opts = run_pass("set x 1.5\nset x [expr {$x + 1}]");
        assert!(
            opts.iter().all(|o| o.code != "O114"),
            "float var must not be rewritten to incr, got {opts:?}",
        );
    }

    #[test]
    fn set_expr_on_untyped_var_is_not_incr() {
        // No prior definition → `x` is not provably INT at the use point;
        // the unsound rewrite is suppressed (matches Python).
        let opts = run_pass("set x [expr {$x + 1}]");
        assert!(
            opts.iter().all(|o| o.code != "O114"),
            "untyped var must not be rewritten to incr, got {opts:?}",
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
        let opts = run_pass("set x 0\nset x [expr {1 + $x}]");
        assert_eq!(
            opts.iter().find(|o| o.code == "O114").unwrap().replacement,
            "incr x",
        );
    }

    #[test]
    fn multi_set_packing_hints_with_three_or_more_literal_sets() {
        let opts = run_pass("set a 1\nset b 2\nset c 3");
        assert!(
            opts.iter().any(|o| o.code == "O119" && o.hint_only),
            "expected O119 hint for multi-set packing, got {opts:?}",
        );
    }

    #[test]
    fn multi_set_packing_ignores_non_literal_values() {
        // Mix of literal + dynamic values — not a pack candidate.
        let opts = run_pass("set a 1\nset b $x\nset c 3");
        assert!(
            opts.iter().all(|o| o.code != "O119"),
            "expected no O119 for mixed run, got {opts:?}",
        );
    }

    #[test]
    fn string_build_chain_folds_via_pattern_recognition() {
        // The pass now emits an *applied* O104 fold (not hint-only) — the
        // detailed behaviour lives in `chain_fold`'s own tests.
        let opts = run_pass("set s {}\nappend s foo\nappend s bar");
        assert!(
            opts.iter()
                .any(|o| o.code == "O104" && !o.hint_only && o.replacement == "set s foobar"),
            "expected applied O104 fold, got {opts:?}",
        );
    }

    #[test]
    fn run_passes_dispatches_pattern_recognition() {
        let cu = CompilationUnit::build_for("set x 0\nset x [expr {$x + 1}]", &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        super::super::run_passes(&mut ctx, &cu, &[super::super::PassId::PatternRecognition]);
        assert!(
            ctx.optimisations.iter().any(|o| o.code == "O114"),
            "expected O114 via run_passes, got {:?}",
            ctx.optimisations,
        );
    }
}
