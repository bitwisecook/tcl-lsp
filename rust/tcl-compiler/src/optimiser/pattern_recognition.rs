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

//! Pattern-recognition optimiser pass.
//!
//! Entry points:
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
use tcl_core_types::DiagCode;

use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::expr_ast::{BinOp, ExprNode};
use crate::ir::{Script, Statement};
use crate::naming::normalise_var_name;
use crate::types::{TclType, TypeKind, TypeLattice};

use super::helpers::literals::{is_safe_word, is_static_var_word};
use super::helpers::spans::{full_rewrite_span, statement_delete_rewrite_range};
use super::{Optimisation, PassContext};

/// Run the pattern-recognition pass.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    let top_ints = int_var_names(&cu.top_level);
    walk_script(ctx, &cu.ir_module.top_level, &top_ints, 0);
    for (qname, proc) in &cu.ir_module.procedures {
        let ints = cu
            .procedures
            .get(qname)
            .map(int_var_names)
            .unwrap_or_default();
        walk_script(ctx, &proc.body, &ints, 0);
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
/// (O114) is suppressed — the loop variable must be proven
/// `INT` (not `DOUBLE` / `NUMERIC` / `BOOLEAN`) at the use point before
/// rewriting. `expr {$x + 1}` silently promotes a float operand, whereas
/// `incr` errors, so the function-level join (all versions must be `INT`) is a
/// sound over-approximation of the per-use check.
fn int_var_names(fu: &FunctionUnit) -> HashSet<String> {
    use std::collections::HashMap;
    let mut acc: HashMap<crate::ssa::Symbol, bool> = HashMap::new();
    for ((sym, _ver), lattice) in &fu.types {
        let is_int = lattice_is_int(lattice);
        acc.entry(*sym)
            .and_modify(|v| *v = *v && is_int)
            .or_insert(is_int);
    }
    acc.into_iter()
        .filter(|(_, ok)| *ok)
        .map(|(sym, _)| fu.ssa.var_name(sym).to_owned())
        .collect()
}

/// Whether a type-lattice element is a known `TclType::Int`.
fn lattice_is_int(t: &TypeLattice) -> bool {
    t.kind() == TypeKind::Known && t.tcl_type() == Some(TclType::Int)
}

/// `depth` is the nesting level of `script` — see
/// [`super::MAX_OPTIMISER_WALK_DEPTH`].
fn walk_script(ctx: &mut PassContext<'_>, script: &Script, int_vars: &HashSet<String>, depth: u32) {
    if super::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
        return;
    }
    detect_multi_set_packing(ctx, script);
    for stmt in &script.statements {
        walk_statement(ctx, stmt, int_vars, depth);
    }
}

/// Minimum number of `set`s to pack into a `lassign` / `foreach`.
const SET_PACK_MIN_GROUP: usize = 3;

/// O119 — pack three or more consecutive `set VAR LITERAL` statements
/// (distinct variables, safe literal values) into one `lassign` (Tcl
/// 8.5 / 8.6) or `foreach {…} {…} {break}` (8.4), emitting the applied
/// rewrite plus paired deletions. Skipped on Tcl 9.0, where individual
/// `set`s are faster. Handles only the strictly-consecutive case;
/// interspersed candidates are not reordered.
fn detect_multi_set_packing(ctx: &mut PassContext<'_>, script: &Script) {
    // Tcl 9.0 prefers individual `set`s; 8.5 / 8.6 get `lassign`; older
    // (and dialect-unset) fall back to the universally-valid `foreach`.
    if ctx.dialect == Some("tcl9.0") {
        return;
    }
    let use_lassign = matches!(ctx.dialect, Some("tcl8.5" | "tcl8.6"));
    let stmts = &script.statements;

    let mut i = 0;
    while i < stmts.len() {
        // Gather a maximal run of consecutive static sets with distinct,
        // non-cross-event variables.
        let mut run: Vec<(usize, String, String)> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        let mut j = i;
        while j < stmts.len() {
            let Some((var_key, var_word, value)) = static_packing_set(&stmts[j]) else {
                break;
            };
            if ctx.cross_event_vars.contains(&var_key) || !seen.insert(var_key) {
                break;
            }
            run.push((j, var_word, value));
            j += 1;
        }
        if run.len() >= SET_PACK_MIN_GROUP {
            emit_set_pack(ctx, stmts, &run, use_lassign);
        }
        i = if j > i { j } else { i + 1 };
    }
}

/// Extract a packable static `set var literal` as `(var_key, var_word,
/// value)`. Handles both lowering shapes: an integer literal lowers to
/// `AssignConst`, other static literals to `AssignValue` (whose value word
/// must be a single static `Esc`/`Str` token with no back-substitution).
fn static_packing_set(stmt: &Statement) -> Option<(String, String, String)> {
    let (name, value) = match stmt {
        Statement::AssignConst { name, value, .. } => (name, value),
        Statement::AssignValue {
            name,
            value,
            value_needs_backsubst,
            tokens,
            ..
        } => {
            if *value_needs_backsubst {
                return None;
            }
            let tokens = tokens.as_ref()?;
            let kind = tokens.argv_kinds.get(2)?;
            if !tokens.single_token_word.get(2).copied().unwrap_or(false)
                || !matches!(kind, tcl_lexer::TokenType::Esc | tcl_lexer::TokenType::Str)
            {
                return None;
            }
            (name, value)
        }
        _ => return None,
    };
    if !is_static_var_word(name) || !is_safe_word(value) {
        return None;
    }
    Some((
        normalise_var_name(name).to_owned(),
        name.clone(),
        value.clone(),
    ))
}

/// Emit the O119 pack rewrite (over the last set) + deletions of the
/// earlier sets, sharing one group.
fn emit_set_pack(
    ctx: &mut PassContext<'_>,
    stmts: &[Statement],
    run: &[(usize, String, String)],
    use_lassign: bool,
) {
    let source = ctx.source;
    let group = ctx.alloc_group();
    let var_words = run
        .iter()
        .map(|(_, w, _)| w.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let value_words = run
        .iter()
        .map(|(_, _, v)| v.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let (replacement, pack_msg, del_msg) = if use_lassign {
        (
            format!("lassign {{{value_words}}} {var_words}"),
            "Pack set statements into lassign",
            "Remove packed set (moved to lassign)",
        )
    } else {
        (
            format!("foreach {{{var_words}}} {{{value_words}}} {{break}}"),
            "Pack set statements into foreach",
            "Remove packed set (moved to foreach)",
        )
    };

    let last_idx = run.last().unwrap().0;
    let mut pack = Optimisation::new(
        DiagCode::O119,
        pack_msg,
        full_rewrite_span(source, stmts[last_idx].span()),
        replacement,
    );
    pack.group = Some(group);
    ctx.report(pack);

    for (idx, _, _) in &run[..run.len() - 1] {
        let full = full_rewrite_span(source, stmts[*idx].span());
        let next_start = stmts.get(idx + 1).map(|s| s.span().start() as usize);
        let mut del = Optimisation::new(
            DiagCode::O119,
            del_msg,
            statement_delete_rewrite_range(source, full, next_start),
            "",
        );
        del.group = Some(group);
        ctx.report(del);
    }
}

fn walk_statement(
    ctx: &mut PassContext<'_>,
    stmt: &Statement,
    int_vars: &HashSet<String>,
    depth: u32,
) {
    match stmt {
        Statement::AssignExpr {
            span, name, expr, ..
        } => {
            if let Some(replacement) = try_incr_idiom(name, expr, int_vars) {
                ctx.report(Optimisation::new(
                    DiagCode::O114,
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
                walk_script(ctx, &c.body, int_vars, depth + 1);
            }
            if let Some(b) = else_body {
                walk_script(ctx, b, int_vars, depth + 1);
            }
        }
        Statement::For {
            init, next, body, ..
        } => {
            walk_script(ctx, init, int_vars, depth + 1);
            walk_script(ctx, next, int_vars, depth + 1);
            walk_script(ctx, body, int_vars, depth + 1);
        }
        Statement::While { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Foreach { body, .. } => walk_script(ctx, body, int_vars, depth + 1),
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_script(ctx, body, int_vars, depth + 1);
            for h in handlers {
                walk_script(ctx, &h.body, int_vars, depth + 1);
            }
            if let Some(fb) = finally_body {
                walk_script(ctx, fb, int_vars, depth + 1);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    walk_script(ctx, b, int_vars, depth + 1);
                }
            }
            if let Some(b) = default_body {
                walk_script(ctx, b, int_vars, depth + 1);
            }
        }
        _ => {}
    }
}

/// If `expr` is of the shape `$var ± literal` (where `var`
/// normalises to `target_name`), return the equivalent `incr`
/// command text.
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

    // helpers

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

    // end-to-end tests

    #[test]
    fn set_expr_plus_one_rewrites_to_incr() {
        // The seeding `set x 0` makes `x` provably INT, satisfying the
        // O114 soundness gate.
        let opts = run_pass("set x 0\nset x [expr {$x + 1}]");
        let got = opts.iter().find(|o| o.code == DiagCode::O114);
        assert!(got.is_some(), "expected O114, got {opts:?}");
        assert_eq!(got.unwrap().replacement, "incr x");
    }

    #[test]
    fn set_expr_plus_n_carries_the_amount() {
        let opts = run_pass("set x 0\nset x [expr {$x + 5}]");
        assert_eq!(
            opts.iter()
                .find(|o| o.code == DiagCode::O114)
                .unwrap()
                .replacement,
            "incr x 5",
        );
    }

    #[test]
    fn set_expr_minus_one_becomes_incr_negative_one() {
        let opts = run_pass("set x 5\nset x [expr {$x - 1}]");
        assert_eq!(
            opts.iter()
                .find(|o| o.code == DiagCode::O114)
                .unwrap()
                .replacement,
            "incr x -1",
        );
    }

    #[test]
    fn set_expr_on_float_var_is_not_incr() {
        // D5-O114: `x` is DOUBLE here, so `expr {$x + 1}` promotes to a
        // float while `incr` would error — the rewrite must not fire.
        let opts = run_pass("set x 1.5\nset x [expr {$x + 1}]");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O114),
            "float var must not be rewritten to incr, got {opts:?}",
        );
    }

    #[test]
    fn set_expr_on_untyped_var_is_not_incr() {
        // No prior definition → `x` is not provably INT at the use point;
        // the unsound rewrite is suppressed.
        let opts = run_pass("set x [expr {$x + 1}]");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O114),
            "untyped var must not be rewritten to incr, got {opts:?}",
        );
    }

    #[test]
    fn set_expr_other_var_is_not_incr() {
        let opts = run_pass("set x [expr {$y + 1}]");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O114),
            "different variable should not be recognised, got {opts:?}",
        );
    }

    #[test]
    fn set_expr_not_add_or_sub_is_ignored() {
        let opts = run_pass("set x [expr {$x * 2}]");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O114),
            "multiplication should not be recognised, got {opts:?}",
        );
    }

    #[test]
    fn commutative_add_accepts_literal_on_left() {
        // Tcl allows `$x + 1` and `1 + $x` — both should be
        // recognised as an incr idiom.
        let opts = run_pass("set x 0\nset x [expr {1 + $x}]");
        assert_eq!(
            opts.iter()
                .find(|o| o.code == DiagCode::O114)
                .unwrap()
                .replacement,
            "incr x",
        );
    }

    #[test]
    fn multi_set_packing_applies_pack_rewrite() {
        // No dialect set → universally-valid `foreach` packing, applied.
        let opts = run_pass("set a 1\nset b 2\nset c 3");
        let pack = opts
            .iter()
            .find(|o| o.code == DiagCode::O119 && !o.replacement.is_empty())
            .expect("expected an applied O119 pack");
        assert_eq!(pack.replacement, "foreach {a b c} {1 2 3} {break}");
        // One pack + two deletions, one group.
        let o119: Vec<_> = opts.iter().filter(|o| o.code == DiagCode::O119).collect();
        assert_eq!(o119.len(), 3);
    }

    #[test]
    fn multi_set_packing_uses_lassign_on_tcl86() {
        let cu = CompilationUnit::build_for("set a 1\nset b 2\nset c 3", &registry(), false);
        let mut ctx = super::super::PassContext::with_dialect(
            &cu.source,
            InterproceduralAnalysis::default(),
            Some("tcl8.6"),
        );
        run(&mut ctx, &cu);
        assert!(
            ctx.optimisations
                .iter()
                .any(|o| o.code == DiagCode::O119 && o.replacement == "lassign {1 2 3} a b c"),
            "expected lassign pack on 8.6, got {:?}",
            ctx.optimisations,
        );
    }

    #[test]
    fn multi_set_packing_skipped_on_tcl9() {
        let cu = CompilationUnit::build_for("set a 1\nset b 2\nset c 3", &registry(), false);
        let mut ctx = super::super::PassContext::with_dialect(
            &cu.source,
            InterproceduralAnalysis::default(),
            Some("tcl9.0"),
        );
        run(&mut ctx, &cu);
        assert!(
            ctx.optimisations.iter().all(|o| o.code != DiagCode::O119),
            "Tcl 9.0 must not pack sets, got {:?}",
            ctx.optimisations,
        );
    }

    #[test]
    fn multi_set_packing_ignores_non_literal_values() {
        // Mix of literal + dynamic values — not a pack candidate.
        let opts = run_pass("set a 1\nset b $x\nset c 3");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O119),
            "expected no O119 for mixed run, got {opts:?}",
        );
    }

    #[test]
    fn string_build_chain_folds_via_pattern_recognition() {
        // The pass now emits an *applied* O104 fold (not hint-only) — the
        // detailed behaviour lives in `chain_fold`'s own tests.
        let opts = run_pass("set s {}\nappend s foo\nappend s bar");
        assert!(
            opts.iter().any(|o| o.code == DiagCode::O104
                && !o.hint_only
                && o.replacement == "set s foobar"),
            "expected applied O104 fold, got {opts:?}",
        );
    }

    #[test]
    fn run_passes_dispatches_pattern_recognition() {
        let cu = CompilationUnit::build_for("set x 0\nset x [expr {$x + 1}]", &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        super::super::run_passes(&mut ctx, &cu, &[super::super::PassId::PatternRecognition]);
        assert!(
            ctx.optimisations.iter().any(|o| o.code == DiagCode::O114),
            "expected O114 via run_passes, got {:?}",
            ctx.optimisations,
        );
    }
}
