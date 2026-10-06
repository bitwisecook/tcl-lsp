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

use crate::compilation_unit::CompilationUnit;
use crate::ir::{Script, Statement};
use crate::naming::normalise_var_name;

use super::helpers::literals::{is_safe_word, is_static_var_word};
use super::helpers::spans::{full_rewrite_span, statement_delete_rewrite_range};
use super::{Optimisation, PassContext};

/// Run the pattern-recognition pass.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    // O119 moves and deletes `set` statements, so a function with a
    // computed variable name (`set $name …`) abstains from packing —
    // the shared value-motion barrier. O114 (`set`/`expr`
    // → `incr`) rewrites a statement in place and stays on.
    let top_pack = !cu.top_level.dynamic_barrier_blocks_value_motion();
    walk_script(ctx, &cu.ir_module.top_level, top_pack, 0);
    for (qname, proc) in &cu.ir_module.procedures {
        let fu = cu.procedures.get(qname);
        let pack = fu.is_some_and(|f| !f.dynamic_barrier_blocks_value_motion());
        walk_script(ctx, &proc.body, pack, 0);
    }
    // O128 — end-offset index rewrites (its own segment-level walk over
    // the same source).
    super::end_offset::run(ctx, cu);
    // O104 / O130 — applied write-only build-chain folds (replaces the old
    // hint-only detector; owns its own per-function escape gate).
    super::chain_fold::run(ctx, cu);
}

/// `depth` is the nesting level of `script` — see
/// [`super::MAX_OPTIMISER_WALK_DEPTH`]. `pack` gates the O119 multi-`set`
/// packing (off for a function whose dynamic-name barrier blocks value
/// motion — see [`run`]).
fn walk_script(ctx: &mut PassContext<'_>, script: &Script, pack: bool, depth: u32) {
    if super::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) || !script.is_authored_source() {
        return;
    }
    if pack {
        detect_multi_set_packing(ctx, script);
    }
    for stmt in &script.statements {
        walk_statement(ctx, script, stmt, pack, depth);
    }
}

/// Minimum number of `set`s to pack into a `lassign` / `foreach`.
const SET_PACK_MIN_GROUP: usize = 3;

/// O119 — pack three or more consecutive `set VAR LITERAL` statements
/// (distinct variables, safe literal values) into one `lassign` (Tcl
/// 8.5 / 8.6) or `foreach {…} {…} {break}` (8.4), retaining hints and paired suggestions until the
/// shared grouped-store proof closes completion, effects and object sharing. Skipped on Tcl 9.0, where individual
/// `set`s are faster. Handles only the strictly-consecutive case;
/// interspersed candidates are not reordered.
fn detect_multi_set_packing(ctx: &mut PassContext<'_>, script: &Script) {
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
        if run.len() >= SET_PACK_MIN_GROUP
            && let Some(assessment) =
                super::store_packing::assess_grouped_store_rewrite(script, i, run.last().unwrap().0)
        {
            emit_set_pack(ctx, stmts, &run, assessment);
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
    assessment: super::store_packing::StorePackingAssessment,
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
    let (replacement, pack_msg, del_msg) =
        if assessment.target == super::store_packing::StorePackingTarget::Lassign {
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
    pack.hint_only = true;
    pack.message.push_str(match assessment.decline {
        super::store_packing::StorePackingDecline::EnclosingResult => {
            "; final command result would change"
        }
        super::store_packing::StorePackingDecline::UnknownResultUse => {
            "; enclosing result use is unproved"
        }
        super::store_packing::StorePackingDecline::UnprovedOutputSchedule => {
            "; native store schedule equivalence is unproved"
        }
    });
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
        del.hint_only = true;
        ctx.report(del);
    }
}

fn walk_statement(
    ctx: &mut PassContext<'_>,
    script: &Script,
    stmt: &Statement,
    pack: bool,
    depth: u32,
) {
    match stmt {
        Statement::AssignExpr { span, .. }
        | Statement::AssignValue { span, .. }
        | Statement::Call { span, .. } => {
            if let Some(proof) = ctx.registry.and_then(|registry| {
                crate::increment_rewrite::assess_increment_rewrite(script, stmt, registry)
            }) {
                let replacement = proof.replacement();
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
                walk_script(ctx, &c.body, pack, depth + 1);
            }
            if let Some(b) = else_body {
                walk_script(ctx, b, pack, depth + 1);
            }
        }
        Statement::For {
            init, next, body, ..
        } => {
            walk_script(ctx, init, pack, depth + 1);
            walk_script(ctx, next, pack, depth + 1);
            walk_script(ctx, body, pack, depth + 1);
        }
        Statement::While { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Foreach { body, .. } => walk_script(ctx, body, pack, depth + 1),
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_script(ctx, body, pack, depth + 1);
            for h in handlers {
                walk_script(ctx, &h.body, pack, depth + 1);
            }
            if let Some(fb) = finally_body {
                walk_script(ctx, fb, pack, depth + 1);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    walk_script(ctx, b, pack, depth + 1);
                }
            }
            if let Some(b) = default_body {
                walk_script(ctx, b, pack, depth + 1);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
fn var_matches(target: &str, candidate: &str) -> bool {
    crate::naming::normalise_var_name(&format!("${candidate}")) == target
}

#[cfg(test)]
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
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap())
    }

    fn run_pass(source: &str) -> Vec<Optimisation> {
        let registry = registry();
        let cu = CompilationUnit::build_for(source, &registry, false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        ctx.registry = Some(&registry);
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

    /// A computed variable name in the proc means the packed
    /// `set`s' targets are not provably distinct from whatever `set $name …`
    /// touches, so O119 abstains for the whole function. The in-place O114
    /// rewrite is unaffected (covered by the tests below).
    #[test]
    fn dynamic_name_write_blocks_multi_set_packing() {
        let opts = run_pass("proc ::f {name} { set $name q\nset a 1\nset b 2\nset c 3 }");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O119),
            "dynamic-name proc must not pack, got {opts:?}",
        );
    }

    /// Control for the gate above: the same run of `set`s without the
    /// dynamic write still packs.
    #[test]
    fn multi_set_packing_still_fires_without_dynamic_name() {
        let opts = run_pass("proc ::f {} { set a 1\nset b 2\nset c 3 }");
        assert!(
            opts.iter().any(|o| o.code == DiagCode::O119),
            "control: clean proc must still pack, got {opts:?}",
        );
    }

    #[test]
    fn set_expr_plus_one_rewrites_to_incr() {
        for (normalise, expected) in [
            ("llength $x\n", "expr {+$x}; incr x"),
            ("incr x 0\n", "incr x"),
        ] {
            let opts = run_pass(&format!("set x 0\n{normalise}set x [expr {{$x + 1}}]"));
            let edit = opts
                .iter()
                .find(|edit| edit.code == DiagCode::O114)
                .expect("the closed source update still supplies an edit");
            assert!(!edit.hint_only);
            assert_eq!(edit.replacement, expected);
        }
    }

    #[test]
    fn set_expr_plus_n_carries_the_amount() {
        for (normalise, expected) in [
            ("llength $x\n", "expr {+$x}; incr x [expr {5}]"),
            ("incr x 0\n", "incr x [expr {5}]"),
        ] {
            let opts = run_pass(&format!("set x 0\n{normalise}set x [expr {{$x + 5}}]"));
            let edit = opts
                .iter()
                .find(|edit| edit.code == DiagCode::O114)
                .expect("the original amount conversion remains executed");
            assert!(!edit.hint_only);
            assert_eq!(edit.replacement, expected);
        }
    }

    #[test]
    fn increment_rewrite_retains_original_nonunit_literal_conversion() {
        for (normalise, expected) in [
            ("llength $x\n", "expr {+$x}; incr x [expr {- [expr {5}]}]"),
            ("incr x 0\n", "incr x [expr {- [expr {5}]}]"),
        ] {
            let opts = run_pass(&format!(
                "set held 5\nset x 9\n{normalise}set x [expr {{$x - 5}}]"
            ));
            let edit = opts
                .iter()
                .find(|edit| edit.code == DiagCode::O114)
                .expect("closed integer update still supplies an applied edit");
            assert!(!edit.hint_only);
            assert_eq!(edit.replacement, expected);
        }
    }

    #[test]
    fn unknown_stock_cache_cannot_authorise_increment_conversion() {
        for source in [
            "set x 0; set x [expr {$x+1}]",
            "set x 0; set x [expr {$x+5}]",
            "set x 9; set x [expr {$x-5}]",
            "set x 5; set x [expr {$x-1}]",
            "set x 0; set x [expr {1+$x}]",
        ] {
            let edits = run_pass(source);
            assert!(
                edits.iter().all(|edit| edit.code != DiagCode::O114),
                "{source}: {edits:?}"
            );
        }
    }

    #[test]
    fn increment_rewrite_requires_original_handlers_and_unobserved_physical_schedule() {
        for source in [
            "proc observer {args} {}; set x 0; trace add variable x read observer; set x [expr {$x+1}]",
            "proc observer {args} {}; set x 0; trace add variable x write observer; set x [expr {$x+1}]",
            "proc observer {args} {}; set x 0; trace add execution expr enter observer; set x [expr {$x+1}]",
            "proc incr {args} {return WRONG}; set x 0; set x [expr {$x+1}]",
            "rename incr original_incr; interp alias {} incr {} original_incr other; set x 0; set x [expr {$x+1}]",
            "set a(k) 0; set a(j) [expr {$a(k)+1}]",
        ] {
            let edits = run_pass(source);
            assert!(
                edits.iter().all(|edit| edit.code != DiagCode::O114),
                "{source}: {edits:?}"
            );
        }
    }

    #[test]
    fn set_expr_minus_one_becomes_incr_negative_one() {
        for (normalise, expected) in [
            ("llength $x\n", "expr {+$x}; incr x -1"),
            ("incr x 0\n", "incr x -1"),
        ] {
            let opts = run_pass(&format!("set x 5\n{normalise}set x [expr {{$x - 1}}]"));
            let edit = opts
                .iter()
                .find(|edit| edit.code == DiagCode::O114)
                .expect("the closed subtraction still supplies an edit");
            assert!(!edit.hint_only);
            assert_eq!(edit.replacement, expected);
        }
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
        for (normalise, expected) in [
            ("llength $x\n", "expr {+$x}; incr x"),
            ("incr x 0\n", "incr x"),
        ] {
            let opts = run_pass(&format!("set x 0\n{normalise}set x [expr {{1 + $x}}]"));
            let edit = opts
                .iter()
                .find(|edit| edit.code == DiagCode::O114)
                .expect("commutative addition retains its single physical read");
            assert!(!edit.hint_only);
            assert_eq!(edit.replacement, expected);
        }
    }

    #[test]
    fn multi_set_packing_keeps_unproved_final_result_as_a_hint() {
        // Actual C8.6 selects lassign, whose empty result differs from final set3.
        let opts = run_pass("set a 1\nset b 2\nset c 3");
        let pack = opts
            .iter()
            .find(|o| o.code == DiagCode::O119 && !o.replacement.is_empty())
            .expect("expected an O119 candidate");
        assert_eq!(pack.replacement, "lassign {1 2 3} a b c");
        assert!(pack.hint_only);
        assert!(pack.message.contains("final command result would change"));
        // One pack + two deletions, one group.
        let o119: Vec<_> = opts.iter().filter(|o| o.code == DiagCode::O119).collect();
        assert_eq!(o119.len(), 3);
        assert!(o119.iter().all(|suggestion| suggestion.hint_only));
    }

    #[test]
    fn multi_set_packing_uses_lassign_on_tcl86() {
        let cu = CompilationUnit::build_for("set a 1\nset b 2\nset c 3", &registry(), false);
        let mut ctx = super::super::PassContext::with_dialect(
            &cu.source,
            InterproceduralAnalysis::default(),
            Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()),
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
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let selected = CommandRegistry::build_default().project_for_profile(profile);
        let cu = CompilationUnit::build_for_profile(
            "set a 1\nset b 2\nset c 3",
            &selected,
            false,
            profile,
        );
        let mut ctx = super::super::PassContext::with_dialect(
            &cu.source,
            InterproceduralAnalysis::default(),
            Some(profile),
        );
        run(&mut ctx, &cu);
        assert!(
            ctx.optimisations.iter().all(|o| o.code != DiagCode::O119),
            "Tcl 9.0 must not pack sets, got {:?}",
            ctx.optimisations,
        );
    }

    #[test]
    fn discarded_result_does_not_prove_grouped_store_schedule() {
        let opts = run_pass("set a 1\nset b 2\nset c 3\nputs $c");
        let candidate = opts
            .iter()
            .find(|item| item.code == DiagCode::O119 && !item.replacement.is_empty())
            .expect("native candidate");
        assert!(candidate.hint_only);
        assert!(
            candidate
                .message
                .contains("store schedule equivalence is unproved")
        );
    }

    #[test]
    fn replaced_packing_handler_cannot_donate_a_native_candidate() {
        let opts = run_pass("proc lassign {args} {return CUSTOM}; set a 1; set b 2; set c 3");
        assert!(opts.iter().all(|item| item.code != DiagCode::O119));
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
        let registry = registry();
        let cu = CompilationUnit::build_for("set x 0\nset x [expr {$x + 1}]", &registry, false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        ctx.registry = Some(&registry);
        super::super::run_passes(&mut ctx, &cu, &[super::super::PassId::PatternRecognition]);
        assert!(
            ctx.optimisations.iter().any(|o| o.code == DiagCode::O114),
            "expected O114 via run_passes, got {:?}",
            ctx.optimisations,
        );
    }
}
