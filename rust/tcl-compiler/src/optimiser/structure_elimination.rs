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

//! Structure-elimination optimiser pass — O112.
//!
//! Walks the structured IR and suggests rewrites where the
//! condition of a compound statement is a compile-time constant:
//!
//! - `if {K}` with a constant-true clause → replace the whole
//!   statement with that clause's body.
//! - `if` where every clause is constant-false → replace with the
//!   `else` body (if any) or empty.
//! - `while {0}` → delete (dead loop).
//! - `for {init} {0} {next} {body}` → delete, keeping the init
//!   script (statements in `init` run once).
//! - `switch` whose selection the solver decided → replace with the body
//!   that runs (a fall-through arm's is the next arm's that has one), or
//!   delete it when no body runs.
//!
//! All rewrites emit diagnostic code `O112`. A `switch`'s selection is the
//! solver's, never a private match: an opaque form (`-glob`, `-regexp`,
//! `-nocase`, a fall-through arm, `case`) reads the unit's selection record
//! ([`crate::sccp::SelectionRecord`]), and the flattened exact form the
//! decided branches of its dispatch chain. A statement the solver never
//! reached — inside an opaque `catch` body — has neither and is left alone.
//!
//! Conditions are decided on the shared expression route
//! ([`crate::value_transfer::decide_condition_detached`]) under the
//! rewrite's whole-module trust, so a condition is decided only where the
//! lattice would decide it — never over a math function the module
//! rebinds or the target lacks, nor past the target's tower. Its
//! variables read an [`Env`] seeded with the per-function SCCP lattice
//! projection: every variable whose lattice entries all agree on the same
//! `Const` value becomes an [`EnvValue`] binding.
//!
//! The pass is driven from a [`CompilationUnit`] because
//! per-function SCCP values come from the [`FunctionUnit`]
//! bundle; the per-function loop lives in the [`run`] entry
//! point.

use std::collections::HashMap;
use tcl_core_types::DiagCode;

use crate::analyses::{ConstValue, LatticeValue};
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::expr_ast::ExprNode;
use crate::ir::{Script, Statement, SwitchArm};
use crate::sccp::BranchFactKind;
use crate::tcl_expr_eval::{Env, EnvValue, FoldPolicy, leading_zero_is_octal};
use tcl_registry::value_transfer::ExactValue;

use super::helpers::spans::full_rewrite_span;
use super::helpers::tokens::extract_body_text;
use super::{Optimisation, PassContext};

/// Run the structure-elimination pass across every function in
/// `cu` — walks the top-level IR script and each
/// procedure body, evaluating each compound-statement condition
/// against the per-function SCCP lattice.
///
/// No trace/alias filtering is applied to the projected `Env`: `sccp()`
/// itself already forces a traced or frame-aliased variable's lattice
/// entry to `Overdefined`, so `sccp_env_for`'s projection is trace/alias
/// safe by construction — O102 `run_load_forwarding` is the one pass that
/// still needs its own check, since it runs an independent def-use-chain
/// scan that never consults `fu.sccp` at all.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    // Top-level script.
    let top_env = sccp_env_for(&cu.top_level);
    let top = Facts {
        env: &top_env,
        fu: &cu.top_level,
    };
    walk_script(ctx, &cu.ir_module.top_level, &top, 0);

    // Procedures.
    for (qname, fu) in &cu.procedures {
        let Some(ir_proc) = cu.ir_module.procedures.get(qname) else {
            continue;
        };
        let env = sccp_env_for(fu);
        walk_script(ctx, &ir_proc.body, &Facts { env: &env, fu }, 0);
    }
}

/// What a function's statements are decided against: the constants its
/// lattice agrees on, for the conditions of `if`, `while` and `for`, and the
/// unit itself, for the selection of a `switch`.
struct Facts<'a> {
    env: &'a Env,
    fu: &'a FunctionUnit,
}

/// Project a function's SCCP lattice into an [`Env`] keyed by
/// variable name. Only variables whose every tracked version
/// collapses to the same `Const` value survive.
fn sccp_env_for(fu: &FunctionUnit) -> Env {
    let mut per_var: HashMap<crate::ssa::Symbol, Vec<&ConstValue>> = HashMap::new();
    let mut dirty: std::collections::HashSet<crate::ssa::Symbol> = std::collections::HashSet::new();
    for ((sym, _ver), lv) in &fu.sccp.values {
        if dirty.contains(sym) {
            continue;
        }
        if let LatticeValue::Const(cv) = lv {
            per_var.entry(*sym).or_default().push(cv);
        } else {
            dirty.insert(*sym);
            per_var.remove(sym);
        }
    }
    let mut env = Env::new();
    for (sym, cvs) in per_var {
        // Require every version to agree on one constant.
        let first = cvs[0];
        if !cvs.iter().all(|cv| *cv == first) {
            continue;
        }
        let entry = match first {
            ConstValue::Int(i) => EnvValue::Int(*i),
            ConstValue::Float(f) => EnvValue::Float(*f),
            ConstValue::Bool(b) => EnvValue::Int(i64::from(*b)),
            ConstValue::String(s) => EnvValue::Str(s.clone()),
        };
        env.insert(fu.ssa.var_name(sym).to_owned(), entry);
    }
    env
}

/// Whether `condition` is decided under `env`: its value on the shared
/// expression route under `ctx`'s target and whole-module trust, read as
/// `if` reads a truth. `None` wherever the route declines — a math function
/// the module rebinds (`proc ::tcl::mathfunc::abs {x} {return 99}` makes
/// `abs(-2) == 2` false from 8.5) or the target lacks, a value past the
/// target's tower (`1 << 70` is 0 under 8.4) — or the value is no truth.
fn decide(ctx: &PassContext<'_>, condition: &ExprNode, env: &Env) -> Option<bool> {
    let constants: HashMap<String, ExactValue> = env
        .iter()
        .map(|(name, value)| {
            let value = match value {
                EnvValue::Int(i) => ConstValue::Int(*i),
                EnvValue::Float(f) => ConstValue::Float(*f),
                EnvValue::Str(s) => ConstValue::String(s.clone()),
            };
            (name.clone(), crate::value_transfer::const_to_exact(&value))
        })
        .collect();
    crate::value_transfer::decide_condition_detached(
        condition,
        &constants,
        ctx.rewrite_folds(),
        FoldPolicy::for_profile(ctx.dialect.and_then(leading_zero_is_octal), ctx.dialect),
    )
}

/// Recursively walk `script`'s statements, trying to eliminate
/// each compound statement and then descending into its bodies.
/// `depth` is the nesting level of `script` — see
/// [`super::MAX_OPTIMISER_WALK_DEPTH`].
fn walk_script(ctx: &mut PassContext<'_>, script: &Script, facts: &Facts<'_>, depth: u32) {
    if super::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
        return;
    }
    for stmt in &script.statements {
        walk_statement(ctx, stmt, facts, depth);
    }
}

fn walk_statement(ctx: &mut PassContext<'_>, stmt: &Statement, facts: &Facts<'_>, depth: u32) {
    match stmt {
        Statement::If { .. } => visit_if(ctx, stmt, facts, depth),
        Statement::While { .. } => visit_while(ctx, stmt, facts, depth),
        Statement::For { .. } => visit_for(ctx, stmt, facts, depth),
        Statement::Switch { .. } => visit_switch(ctx, stmt, facts, depth),
        Statement::Catch { body, .. } | Statement::Foreach { body, .. } => {
            walk_script(ctx, body, facts, depth + 1);
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_script(ctx, body, facts, depth + 1);
            for h in handlers {
                walk_script(ctx, &h.body, facts, depth + 1);
            }
            if let Some(fb) = finally_body {
                walk_script(ctx, fb, facts, depth + 1);
            }
        }
        _ => {}
    }
}

fn visit_if(ctx: &mut PassContext<'_>, stmt: &Statement, facts: &Facts<'_>, depth: u32) {
    let Statement::If {
        span,
        clauses,
        else_body,
        else_span,
    } = stmt
    else {
        return;
    };
    try_eliminate_if(
        ctx,
        *span,
        clauses,
        else_body.as_ref(),
        *else_span,
        facts.env,
    );
    for clause in clauses {
        walk_script(ctx, &clause.body, facts, depth + 1);
    }
    if let Some(body) = else_body {
        walk_script(ctx, body, facts, depth + 1);
    }
}

fn visit_while(ctx: &mut PassContext<'_>, stmt: &Statement, facts: &Facts<'_>, depth: u32) {
    let Statement::While {
        span,
        condition,
        body,
        ..
    } = stmt
    else {
        return;
    };
    if decide(ctx, condition, facts.env) == Some(false) {
        ctx.report(Optimisation::new(
            DiagCode::O112,
            "Eliminate dead while loop (condition is always false)",
            full_rewrite_span(ctx.source, *span),
            "",
        ));
    }
    walk_script(ctx, body, facts, depth + 1);
}

fn visit_for(ctx: &mut PassContext<'_>, stmt: &Statement, facts: &Facts<'_>, depth: u32) {
    let Statement::For {
        span,
        init,
        init_span,
        condition,
        next,
        body,
        ..
    } = stmt
    else {
        return;
    };
    if decide(ctx, condition, facts.env) == Some(false) {
        if init.statements.is_empty() {
            ctx.report(Optimisation::new(
                DiagCode::O112,
                "Eliminate dead for loop (condition is always false)",
                full_rewrite_span(ctx.source, *span),
                "",
            ));
        } else {
            let replacement = extract_body_text(ctx.source, *init_span, *span);
            ctx.report(Optimisation::new(
                DiagCode::O112,
                "Eliminate dead for loop (condition is always false); keep init",
                full_rewrite_span(ctx.source, *span),
                replacement,
            ));
        }
    }
    walk_script(ctx, init, facts, depth + 1);
    walk_script(ctx, body, facts, depth + 1);
    walk_script(ctx, next, facts, depth + 1);
}

fn visit_switch(ctx: &mut PassContext<'_>, stmt: &Statement, facts: &Facts<'_>, depth: u32) {
    let Statement::Switch {
        arms, default_body, ..
    } = stmt
    else {
        return;
    };
    try_eliminate_switch(ctx, stmt, facts.fu);
    for arm in arms {
        if let Some(body) = &arm.body {
            walk_script(ctx, body, facts, depth + 1);
        }
    }
    if let Some(body) = default_body {
        walk_script(ctx, body, facts, depth + 1);
    }
}

fn try_eliminate_if(
    ctx: &mut PassContext<'_>,
    stmt_span: tcl_lexer::Span,
    clauses: &[crate::ir::IfClause],
    else_body: Option<&Script>,
    else_span: Option<tcl_lexer::Span>,
    env: &Env,
) {
    for clause in clauses {
        let Some(truth) = decide(ctx, &clause.condition, env) else {
            return;
        };
        if truth {
            let replacement = extract_body_text(ctx.source, clause.body_span, stmt_span);
            ctx.report(Optimisation::new(
                DiagCode::O112,
                "Eliminate constant if (condition is always true)",
                full_rewrite_span(ctx.source, stmt_span),
                replacement,
            ));
            return;
        }
    }
    // Every clause folded to false.
    if let (Some(body), Some(span)) = (else_body, else_span) {
        let _ = body;
        let replacement = extract_body_text(ctx.source, span, stmt_span);
        ctx.report(Optimisation::new(
            DiagCode::O112,
            "Eliminate constant if (all conditions false); keep else",
            full_rewrite_span(ctx.source, stmt_span),
            replacement,
        ));
    } else {
        ctx.report(Optimisation::new(
            DiagCode::O112,
            "Eliminate dead if (all conditions are always false)",
            full_rewrite_span(ctx.source, stmt_span),
            "",
        ));
    }
}

/// What the solver decided about a `switch`, in the statement's own arm
/// indices: an index one past its arms is the final `default`.
struct Decided {
    /// Per member of the subject, the arm whose pattern matched.
    selected: Vec<Option<usize>>,
    /// The arm whose body runs for every member; `None` when none does.
    body: Option<usize>,
}

/// The selection of the opaque `switch` `stmt` in `fu`'s record: every
/// member of every record at the statement's span must run the same body,
/// and the record must state the statement's own arms. A record that makes
/// stores the rewrite would drop is not folded.
fn record_decision(fu: &FunctionUnit, stmt: &Statement) -> Option<Decided> {
    let Statement::Switch {
        span,
        arms,
        default_body,
        ..
    } = stmt
    else {
        return None;
    };
    let mut selected = Vec::new();
    let mut bodies = Vec::new();
    for record in fu
        .sccp
        .selections
        .iter()
        .filter(|record| fu.abs_span(record.span) == *span)
    {
        let states_the_arms = record.arm_pattern_spans.len() == arms.len()
            && record
                .arm_pattern_spans
                .iter()
                .zip(arms)
                .all(|(recorded, arm)| fu.abs_span(*recorded) == arm.pattern_span);
        if !states_the_arms || record.fact.writes.iter().any(|writes| !writes.is_empty()) {
            return None;
        }
        selected.extend_from_slice(&record.fact.selected);
        bodies.extend_from_slice(&record.fact.bodies);
    }
    let body = *bodies.first()?;
    let runs_default = body.is_some_and(|arm| arm >= arms.len());
    (bodies.iter().all(|other| *other == body)
        && (!runs_default || (body == Some(arms.len()) && default_body.is_some())))
    .then_some(Decided { selected, body })
}

/// The selection of the flattened exact `switch` `stmt` in `fu`: the first
/// arm whose dispatch branch the solver decided true with every earlier one
/// decided false, or the default when all are false. A branch the solver did
/// not decide leaves the selection open.
fn chain_decision(fu: &FunctionUnit, stmt: &Statement) -> Option<Decided> {
    let Statement::Switch {
        arms, default_body, ..
    } = stmt
    else {
        return None;
    };
    // With no arm there is no dispatch branch: nothing states that the
    // subject is read, so the statement keeps its own evaluation of it.
    if arms.is_empty() {
        return None;
    }
    for (index, arm) in arms.iter().enumerate() {
        let decided = fu.sccp.constant_branches.iter().find(|branch| {
            branch.kind == BranchFactKind::Applied
                && branch.span.map(|span| fu.abs_span(span)) == Some(arm.pattern_span)
        })?;
        if decided.value {
            return Some(Decided {
                selected: vec![Some(index)],
                body: Some(index),
            });
        }
    }
    let default = default_body.is_some().then_some(arms.len());
    Some(Decided {
        selected: vec![default],
        body: default,
    })
}

/// The selection the solver decided for `stmt`, whichever way it states it:
/// the dispatch chain's decided branches where the lowering made one, else
/// the selection record. A statement has only one of the two — the lowering
/// (`cfg_builder::switch_is_flattened`) chose its form under the registry's
/// release, which this pass does not see — so the facts there are the answer.
fn decided(fu: &FunctionUnit, stmt: &Statement) -> Option<Decided> {
    chain_decision(fu, stmt).or_else(|| record_decision(fu, stmt))
}

/// The diagnostic's account of a decided selection: the pattern every member
/// matches, the default kept, the dead statement, or — when members match
/// different patterns that share a body — the pattern whose body runs.
fn describe(subject: &str, decided: &Decided, arms: &[SwitchArm]) -> String {
    let pattern = |arm: usize| arms.get(arm).map_or("default", |arm| arm.pattern.as_str());
    let first = decided.selected.first().copied().flatten();
    let matched = first.filter(|_| decided.selected.iter().all(|arm| *arm == first));
    match (decided.body, matched) {
        (None, _) => format!("Eliminate dead switch (subject '{subject}' matches no pattern)"),
        (Some(_), Some(arm)) if arm < arms.len() => format!(
            "Eliminate switch (subject '{subject}' always matches pattern '{}')",
            pattern(arm),
        ),
        (Some(_), Some(_)) => {
            format!("Eliminate switch (subject '{subject}' matches no pattern); keep default")
        }
        (Some(body), None) => format!(
            "Eliminate switch (subject '{subject}' always runs the body of pattern '{}')",
            pattern(body),
        ),
    }
}

/// Replace the `switch` `stmt` with the body the solver decided it always
/// runs, or delete it when it decided none does. A selection the solver did
/// not make — an undecided subject or pattern, a fact declined, members that
/// run different bodies — leaves the statement alone.
fn try_eliminate_switch(ctx: &mut PassContext<'_>, stmt: &Statement, fu: &FunctionUnit) {
    let Statement::Switch {
        span,
        subject,
        arms,
        default_span,
        ..
    } = stmt
    else {
        return;
    };
    let Some(decided) = decided(fu, stmt) else {
        return;
    };
    let replacement = match decided.body {
        None => String::new(),
        Some(arm) => {
            let body_span = arms.get(arm).map_or(*default_span, |arm| arm.body_span);
            let Some(body_span) = body_span else {
                return;
            };
            extract_body_text(ctx.source, body_span, *span)
        }
    };
    ctx.report(Optimisation::new(
        DiagCode::O112,
        describe(subject, &decided, arms),
        full_rewrite_span(ctx.source, *span),
        replacement,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    use tcl_registry::CommandRegistry;

    use crate::compilation_unit::CompilationUnit;
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

    /// The pass over `source` built for `dialect`, whose profile the
    /// selection record of an opaque form needs: a unit built with none
    /// resolves no command's selection.
    fn run_pass_in(source: &str, dialect: &str) -> Vec<Optimisation> {
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let cu = CompilationUnit::build_for_dialect(source, registry, false, dialect);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        run(&mut ctx, &cu);
        ctx.optimisations
    }

    /// The one O112 the pass reports over `source` under `dialect`.
    fn the_fold(source: &str, dialect: &str) -> Optimisation {
        let mut folds: Vec<Optimisation> = run_pass_in(source, dialect)
            .into_iter()
            .filter(|o| o.code == DiagCode::O112)
            .collect();
        assert_eq!(folds.len(), 1, "{dialect}: {source}\n{folds:#?}");
        folds.remove(0)
    }

    // end-to-end tests

    #[test]
    fn constant_true_if_replaces_with_body() {
        let opts = run_pass("if {1} { puts hi } else { puts bye }");
        // Expect one O112 for the if; branch-folding is not run
        // in this test so no O101.
        assert!(
            opts.iter()
                .any(|o| o.code == DiagCode::O112 && o.message.starts_with("Eliminate constant if")),
            "expected an if-elimination O112, got {opts:?}",
        );
    }

    #[test]
    fn constant_false_if_replaces_with_else() {
        let opts = run_pass("if {0} { puts hi } else { puts bye }");
        let opt = opts
            .iter()
            .find(|o| o.code == DiagCode::O112)
            .expect("expected an O112");
        assert!(opt.message.contains("all conditions false"));
    }

    #[test]
    fn constant_false_if_without_else_is_deleted() {
        let opts = run_pass("if {0} { puts hi }");
        let opt = opts
            .iter()
            .find(|o| o.code == DiagCode::O112)
            .expect("expected an O112");
        assert!(opt.message.contains("all conditions are always false"));
        assert_eq!(opt.replacement, "");
    }

    #[test]
    fn while_false_is_dead_loop() {
        let opts = run_pass("while {0} { puts never }");
        assert!(
            opts.iter()
                .any(|o| o.code == DiagCode::O112 && o.message.contains("dead while loop")),
            "expected a dead-while O112, got {opts:?}",
        );
    }

    #[test]
    fn for_false_with_init_keeps_init() {
        let opts = run_pass("for {set i 0} {0} {incr i} { puts $i }");
        let opt = opts
            .iter()
            .find(|o| o.code == DiagCode::O112)
            .expect("expected an O112");
        assert!(opt.message.contains("keep init"));
        assert!(opt.replacement.contains("set i 0"));
    }

    #[test]
    fn for_false_without_init_is_dead() {
        let opts = run_pass("for {} {0} {} { puts $i }");
        let opt = opts
            .iter()
            .find(|o| o.code == DiagCode::O112)
            .expect("expected an O112");
        assert!(opt.message.contains("dead for loop"));
        assert!(!opt.message.contains("keep init"));
        assert_eq!(opt.replacement, "");
    }

    #[test]
    fn switch_literal_subject_matches_arm() {
        let opts = run_pass("switch foo { foo { puts one } bar { puts two } }");
        assert!(
            opts.iter().any(|o| o.code == DiagCode::O112
                && o.message
                    .contains("subject 'foo' always matches pattern 'foo'")),
            "expected a switch-match O112, got {opts:?}",
        );
    }

    #[test]
    fn switch_no_match_emits_default() {
        let opts =
            run_pass("switch baz { foo { puts one } bar { puts two } default { puts none } }");
        assert!(
            opts.iter()
                .any(|o| o.code == DiagCode::O112 && o.message.contains("keep default")),
            "expected a switch-no-match-with-default O112, got {opts:?}",
        );
    }

    #[test]
    fn switch_no_match_no_default_emits_empty() {
        let opts = run_pass("switch baz { foo { puts one } bar { puts two } }");
        assert!(
            opts.iter().any(|o| o.code == DiagCode::O112
                && o.message.contains("matches no pattern")
                && o.replacement.is_empty()),
            "expected a dead-switch O112, got {opts:?}",
        );
    }

    /// A `-regexp` switch over a constant subject folds: the selection is
    /// the registry's, run through the regexp engine, where the pass had
    /// bailed on the mode. tclsh 8.6 prints `ok` for the program.
    #[test]
    fn switch_regexp_mode_selects_through_the_owner() {
        let fold = the_fold("switch -regexp foo { ^foo$ { puts ok } }", "tcl8.6");
        assert_eq!(fold.replacement, "puts ok");
        assert!(fold.message.contains("always matches pattern '^foo$'"));
        let none = the_fold("switch -regexp foo { ^bar$ { puts ok } }", "tcl8.6");
        assert_eq!(none.replacement, "");
        assert!(none.message.contains("matches no pattern"));
    }

    /// Each opaque mode folds through its record: the arm the command
    /// selects is the arm that stays. `-nocase` needs a release that has it,
    /// so a profile naming none leaves it alone.
    #[test]
    fn opaque_modes_fold_to_the_arm_the_command_selects() {
        let cases = [
            (
                "switch -glob abc { a* {puts A} default {puts D} }",
                "puts A",
            ),
            (
                "switch -glob xyz { a* {puts A} default {puts D} }",
                "puts D",
            ),
            ("switch -regexp abc { ^b {puts B} c$ {puts C} }", "puts C"),
            (
                "switch -glob abc { a* - z* {puts B} default {puts D} }",
                "puts B",
            ),
            ("switch abc { abc - def - default {puts D} }", "puts D"),
        ];
        for (source, kept) in cases {
            for dialect in ["tcl8.6", "tcl9.0", "tcl"] {
                assert_eq!(
                    the_fold(source, dialect).replacement,
                    kept,
                    "{dialect}: {source}"
                );
            }
        }
        let nocase = "switch -nocase ABC { abc {puts yes} default {puts no} }";
        for dialect in ["tcl8.6", "tcl9.0"] {
            assert_eq!(
                the_fold(nocase, dialect).replacement,
                "puts yes",
                "{dialect}"
            );
        }
        assert!(
            run_pass_in(nocase, "tcl")
                .iter()
                .all(|o| o.code != DiagCode::O112)
        );
    }

    /// A subject with several versions folds where the flow-insensitive
    /// projection saw none: the record reads the version the statement reads.
    #[test]
    fn the_selection_reads_the_subject_at_the_statement() {
        let build = "set acc \"\"; append acc foo; append acc bar\n";
        for mode in ["", "-glob ", "-regexp ", "-nocase "] {
            let source = format!(
                "{build}switch {mode}-- $acc {{\n    baz     {{ puts never }}\n    default {{ puts always }}\n}}\n"
            );
            let fold = the_fold(&source, "tcl8.6");
            assert_eq!(fold.replacement, "puts always", "{source}");
        }
    }

    /// A finite-set subject folds only when every member runs the same
    /// body; members that part leave the statement alone.
    #[test]
    fn a_finite_subject_folds_only_where_its_members_agree() {
        let choose = "if {$c} {set x a1} else {set x a2}\n";
        let agree = format!(
            "proc p {{c}} {{\n{choose}switch -glob -- $x {{a* {{puts A}} default {{puts D}}}}\n}}"
        );
        assert_eq!(the_fold(&agree, "tcl8.6").replacement, "puts A");
        let part = format!(
            "proc p {{c}} {{\n{choose}switch -glob -- $x {{a1 {{puts A}} default {{puts D}}}}\n}}"
        );
        assert!(
            run_pass_in(&part, "tcl8.6")
                .iter()
                .all(|o| o.code != DiagCode::O112),
            "members that select different bodies must not fold"
        );
        // Different patterns, one shared body through fall-through.
        let shared = format!(
            "proc p {{c}} {{\n{choose}switch -glob -- $x {{a1 - a2 {{puts S}} default {{puts D}}}}\n}}"
        );
        let fold = the_fold(&shared, "tcl8.6");
        assert_eq!(fold.replacement, "puts S");
        assert!(
            fold.message
                .contains("always runs the body of pattern 'a2'"),
            "{fold:?}"
        );
    }

    /// `case` reads its own contract: glob patterns, the `in` word skipped.
    #[test]
    fn case_folds_through_its_own_selection() {
        for clauses in [
            "in a* {puts yes} default {puts no}",
            "{a* {puts yes} default {puts no}}",
        ] {
            let source = format!("case abc {clauses}");
            for dialect in ["tcl8.4", "tcl8.6", "f5-irules"] {
                assert_eq!(
                    the_fold(&source, dialect).replacement,
                    "puts yes",
                    "{dialect}: {source}"
                );
            }
            assert!(
                run_pass_in(&source, "tcl9.0")
                    .iter()
                    .all(|o| o.code != DiagCode::O112),
                "there is no `case` from 9.0"
            );
        }
    }

    /// A selection the owner declines is not made up here: a malformed
    /// pattern, a quoted `-` body of the separate-words form where 9.1 reads
    /// it two ways, a subject the lattice does not know, and a statement
    /// with only a `default` arm, whose subject nothing states is read. The
    /// clause-list form reads its elements by content on every path, so it
    /// folds.
    #[test]
    fn a_declined_selection_leaves_the_switch_alone() {
        let quoted = "switch -glob -- abc a* \"-\" z* {puts B} default {puts D}";
        let list = "switch -glob abc { a* \"-\" z* {puts B} default {puts D} }";
        let declined = [
            (
                "switch -regexp abc { {[} {puts x} default {puts y} }",
                "tcl8.6",
            ),
            (quoted, "tcl9.1"),
            (quoted, "tcl"),
            (
                "proc p {s} { switch -glob -- $s { a* {puts A} default {puts D} } }",
                "tcl8.6",
            ),
            ("switch [gets stdin] {default {puts d}}", "tcl8.6"),
            ("switch $x {default {puts d}}", "tcl8.6"),
        ];
        for (source, dialect) in declined {
            assert!(
                run_pass_in(source, dialect)
                    .iter()
                    .all(|o| o.code != DiagCode::O112),
                "{dialect}: {source}"
            );
        }
        for dialect in ["tcl8.6", "tcl9.0"] {
            assert_eq!(the_fold(quoted, dialect).replacement, "puts B", "{dialect}");
        }
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1", "tcl"] {
            assert_eq!(the_fold(list, dialect).replacement, "puts B", "{dialect}");
        }
    }

    /// O112 reads the words of a flattened `switch` by their values: each
    /// program selects its `hit` arm whichever way its subject and pattern are
    /// spelled, where the dispatch chain compared the subject's spelling to the
    /// decoded pattern and kept the default. Each prints `hit` under tclsh 8.4
    /// to 9.1.
    #[test]
    fn a_switch_is_selected_by_the_values_of_its_words() {
        let programs = [
            r#"switch -- a\nb {"a\nb" {puts hit} default {puts miss}}"#,
            r#"switch -exact -- "a\tb" {a\tb {puts hit} default {puts miss}}"#,
            r#"switch a\nb {"a\nb" {puts hit} default {puts miss}}"#,
            r#"switch "a\nb" {a\nb {puts hit} default {puts miss}}"#,
            r#"switch "a\tb" {a\tb {puts hit} default {puts miss}}"#,
            r#"switch a\tb {"a\tb" {puts hit} default {puts miss}}"#,
            r#"switch "a\\b" {{a\b} {puts hit} default {puts miss}}"#,
            r#"switch {a\b} {"a\\b" {puts hit} default {puts miss}}"#,
            "switch \"a\\nb\" {{a\nb} {puts hit} default {puts miss}}",
            "switch {a\nb} {\"a\\nb\" {puts hit} default {puts miss}}",
            "switch {a\\\nb} {{a b} {puts hit} default {puts miss}}",
            r#"switch "a\tb" a\tb {puts hit} default {puts miss}"#,
            r#"switch a\tb "a\tb" {puts hit} default {puts miss}"#,
            r"switch a\$b {a\$b {puts hit} default {puts miss}}",
            r"switch a\[b {a\[b {puts hit} default {puts miss}}",
            "set s \"a\\nb\"\nswitch $s {a\\nb {puts hit} default {puts miss}}",
            r#"switch -glob -- a\nb {"a\nb" {puts hit} default {puts miss}}"#,
            r"switch -glob -- a\$b {a\$b {puts hit} default {puts miss}}",
        ];
        for source in programs {
            for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
                assert_eq!(
                    the_fold(source, dialect).replacement,
                    "puts hit",
                    "{dialect}: {source}"
                );
            }
        }
    }

    /// Before 8.5 `switch` reads every leading word that starts with `-` as an
    /// option, however many words follow, so a subject holding `-glob` is one:
    /// tclsh 8.4 rejects the program with `bad option`, where 8.5 to 9.1 select
    /// the `-glob` arm and print `G`. A release that may be 8.4 leaves the
    /// statement alone; `--` ends the run and a subject that does not start with
    /// `-` is read by no scan, so those fold under every release.
    #[test]
    fn a_subject_a_release_may_read_as_an_option_is_left_alone_before_8_5() {
        let bare = "set x -glob\nswitch $x {-glob {puts G} default {puts D}}\n";
        let escaped = "switch \\x2dglob {-glob {puts G} default {puts D}}\n";
        for source in [bare, escaped] {
            for dialect in ["tcl8.4", "f5-irules", "tk"] {
                assert!(
                    run_pass_in(source, dialect)
                        .iter()
                        .all(|o| o.code != DiagCode::O112),
                    "{dialect}: {source}"
                );
            }
            for dialect in ["tcl8.5", "tcl8.6", "tcl9.0"] {
                assert_eq!(
                    the_fold(source, dialect).replacement,
                    "puts G",
                    "{dialect}: {source}"
                );
            }
        }
        let ended = "set x -glob\nswitch -- $x {-glob {puts G} default {puts D}}\n";
        let plain = "set x a\nswitch $x {a {puts A} default {puts D}}\n";
        for dialect in ["tcl8.4", "tcl8.6", "tk"] {
            assert_eq!(the_fold(ended, dialect).replacement, "puts G", "{dialect}");
            assert_eq!(the_fold(plain, dialect).replacement, "puts A", "{dialect}");
        }
    }

    /// With pattern and body words the subject is inside the option scan on
    /// every release — 8.5 to 9.1 stop the scan with two words left, and a
    /// pattern and its body are two words — so a variable holding `-glob` is an
    /// option there too: every tclsh rejects the program below with `extra
    /// switch pattern with no body`. The statement is left alone under every
    /// profile; a value that does not start with `-` folds through the
    /// statement's selection record, and `--` ends the run.
    #[test]
    fn a_subject_inside_the_scan_of_the_arms_as_words_is_left_alone_on_every_release() {
        let bare = "set x -glob\nswitch $x a {puts A} default {puts D}\n";
        let plain = "set x a\nswitch $x a {puts A} default {puts D}\n";
        let ended = "set x -glob\nswitch -- $x -glob {puts G} default {puts D}\n";
        for dialect in ["tcl8.4", "f5-irules", "tk", "tcl8.5", "tcl8.6", "tcl9.0"] {
            assert!(
                run_pass_in(bare, dialect)
                    .iter()
                    .all(|o| o.code != DiagCode::O112),
                "{dialect}"
            );
            assert_eq!(the_fold(plain, dialect).replacement, "puts A", "{dialect}");
            assert_eq!(the_fold(ended, dialect).replacement, "puts G", "{dialect}");
        }
    }

    #[test]
    fn nested_elimination_runs_on_inner_bodies() {
        let opts = run_pass("if {1} { if {0} { puts never } else { puts here } }");
        // Outer if is constant-true → one O112. Inner if is
        // constant-false with else → another O112.
        let count = opts.iter().filter(|o| o.code == DiagCode::O112).count();
        assert!(
            count >= 2,
            "expected both outer + inner eliminations, got {opts:?}",
        );
    }

    #[test]
    fn sccp_env_extraction_promotes_single_const() {
        let cu = CompilationUnit::build_for("set x 7\nif {$x} { puts ok }", &registry(), false);
        let env = sccp_env_for(&cu.top_level);
        // `x` is constant → present in env.
        assert!(env.contains_key("x"));
    }

    /// A command the module cannot see may rewrite a plain top-level name, the
    /// global name, so a name read after the call is no more constant than its
    /// `::` spelling, and so may it a procedure's local through `upvar 1`. A
    /// name read only before the call keeps its constant.
    #[test]
    fn sccp_env_extraction_leaves_out_a_name_an_unseen_call_may_write() {
        let unseen =
            CompilationUnit::build_for("set x 7\nok\nif {$x} { puts ok }", &registry(), false);
        assert!(!sccp_env_for(&unseen.top_level).contains_key("x"));
        let qualified =
            CompilationUnit::build_for("set ::x 7\nif {$::x} { puts ok }", &registry(), false);
        assert!(!sccp_env_for(&qualified.top_level).contains_key("::x"));
        let local = CompilationUnit::build_for(
            "proc p {} { set x 7; ok; if {$x} { puts ok } }",
            &registry(),
            false,
        );
        assert!(!sccp_env_for(&local.procedures["::p"]).contains_key("x"));
        let before = CompilationUnit::build_for("set x 7\nif {$x} { ok }", &registry(), false);
        assert!(sccp_env_for(&before.top_level).contains_key("x"));
    }

    #[test]
    fn unknown_condition_produces_no_fold() {
        // `$x` has no lattice binding → eval returns None → no
        // O112 from the if. Branch-folding isn't run here.
        let opts = run_pass("if {$x} { ok } else { bad }");
        assert!(opts.iter().all(|o| o.code != DiagCode::O112));
    }

    /// No O112 folds a condition over a name the lattice cannot pin: one an
    /// arm of a `switch` the flow graph keeps as one statement writes, one a
    /// callback script writes, and one a command the module cannot see may
    /// write, as it may write `::g` — a top-level name, or a procedure's local
    /// through `upvar 1`. A name none of them writes and a name a procedure
    /// the module defines leaves alone still fold.
    #[test]
    fn o112_leaves_a_condition_over_a_name_the_lattice_cannot_pin() {
        for source in [
            "set go 1\nswitch -glob -- [gets stdin] { q* { set go 0 } }\nif {$go} { puts a } else { puts b }",
            "set go 1\nswitch -nocase -- [gets stdin] { q { set go 0 } }\nif {$go} { puts a } else { puts b }",
            "set go 1\nswitch -regexp -- [gets stdin] { {^q} { set go 0 } }\nif {$go} { puts a } else { puts b }",
            "set go 1\nswitch -glob -- [gets stdin] { x - q* { set go 0 } }\nif {$go} { puts a } else { puts b }",
            "proc p {} {\n set go 1\n switch -glob -- [gets stdin] { q* { set go 0 } }\n if {$go} { puts a } else { puts b }\n}",
            "set go 1\ntrace add variable x write { set ::go 0 ;# }\nset x 1\nif {$go} { puts a } else { puts b }",
            "set go 1\nafter idle {set ::go 0}\nif {$go} { puts a } else { puts b }",
            "set g 5\nfoo\nif {$g} { puts a } else { puts b }",
            "set ::g 5\nfoo\nif {$::g} { puts a } else { puts b }",
            "proc p {} {\n set g 5\n foo\n if {$g} { puts a } else { puts b }\n}",
        ] {
            assert!(
                run_pass(source).iter().all(|o| o.code != DiagCode::O112),
                "{source}"
            );
        }
        for source in [
            "set go 1\nswitch -glob -- [gets stdin] { q* { set other 0 } }\nif {$go} { puts a } else { puts b }",
            "proc foo {} { puts hi }\nset g 5\nfoo\nif {$g} { puts a } else { puts b }",
        ] {
            assert!(
                run_pass(source).iter().any(|o| o.code == DiagCode::O112),
                "{source}"
            );
        }
    }
}
