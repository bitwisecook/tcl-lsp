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
//! - `switch LITERAL` → replace with the matching arm (following
//!   fall-through chains) or the default / empty.
//!
//! All rewrites emit diagnostic code `O112`. Conditions are
//! evaluated via [`eval_tcl_expr`] against an [`Env`] seeded with
//! the per-function SCCP lattice projection: every variable whose
//! lattice entries all agree on the same `Const` value becomes an
//! [`EnvValue`] binding.
//!
//! The pass is driven from a [`CompilationUnit`] because
//! per-function SCCP values come from the [`FunctionUnit`]
//! bundle; the per-function loop is folded into the
//! `run_function` entry point.

use std::collections::HashMap;
use tcl_core_types::DiagCode;

use crate::analyses::{ConstValue, LatticeValue};
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::{Script, Statement, SwitchArm, SwitchMode};
use crate::naming::normalise_var_name;
use crate::tcl_expr_eval::{Env, EnvValue, eval_tcl_expr_with_octal, leading_zero_is_octal};

use super::helpers::literals::is_plain_literal;
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
    walk_script(ctx, &cu.ir_module.top_level, &top_env);

    // Procedures.
    for (qname, fu) in &cu.procedures {
        let Some(ir_proc) = cu.ir_module.procedures.get(qname) else {
            continue;
        };
        let env = sccp_env_for(fu);
        walk_script(ctx, &ir_proc.body, &env);
    }
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

/// Recursively walk `script`'s statements, trying to eliminate
/// each compound statement and then descending into its bodies.
fn walk_script(ctx: &mut PassContext<'_>, script: &Script, env: &Env) {
    for stmt in &script.statements {
        walk_statement(ctx, stmt, env);
    }
}

fn walk_statement(ctx: &mut PassContext<'_>, stmt: &Statement, env: &Env) {
    match stmt {
        Statement::If { .. } => visit_if(ctx, stmt, env),
        Statement::While { .. } => visit_while(ctx, stmt, env),
        Statement::For { .. } => visit_for(ctx, stmt, env),
        Statement::Switch { .. } => visit_switch(ctx, stmt, env),
        Statement::Catch { body, .. } | Statement::Foreach { body, .. } => {
            walk_script(ctx, body, env);
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_script(ctx, body, env);
            for h in handlers {
                walk_script(ctx, &h.body, env);
            }
            if let Some(fb) = finally_body {
                walk_script(ctx, fb, env);
            }
        }
        _ => {}
    }
}

fn visit_if(ctx: &mut PassContext<'_>, stmt: &Statement, env: &Env) {
    let Statement::If {
        span,
        clauses,
        else_body,
        else_span,
    } = stmt
    else {
        return;
    };
    try_eliminate_if(ctx, *span, clauses, else_body.as_ref(), *else_span, env);
    for clause in clauses {
        walk_script(ctx, &clause.body, env);
    }
    if let Some(body) = else_body {
        walk_script(ctx, body, env);
    }
}

fn visit_while(ctx: &mut PassContext<'_>, stmt: &Statement, env: &Env) {
    let Statement::While {
        span,
        condition,
        body,
        ..
    } = stmt
    else {
        return;
    };
    if let Some(val) =
        eval_tcl_expr_with_octal(condition, env, ctx.dialect.map(leading_zero_is_octal))
        && !val.is_truthy()
    {
        ctx.report(Optimisation::new(
            DiagCode::O112,
            "Eliminate dead while loop (condition is always false)",
            full_rewrite_span(ctx.source, *span),
            "",
        ));
    }
    walk_script(ctx, body, env);
}

fn visit_for(ctx: &mut PassContext<'_>, stmt: &Statement, env: &Env) {
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
    if let Some(val) =
        eval_tcl_expr_with_octal(condition, env, ctx.dialect.map(leading_zero_is_octal))
        && !val.is_truthy()
    {
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
    walk_script(ctx, init, env);
    walk_script(ctx, body, env);
    walk_script(ctx, next, env);
}

fn visit_switch(ctx: &mut PassContext<'_>, stmt: &Statement, env: &Env) {
    let Statement::Switch {
        span,
        subject,
        arms,
        default_body,
        default_span,
        mode,
        nocase,
        ..
    } = stmt
    else {
        return;
    };
    try_eliminate_switch(
        ctx,
        *span,
        &SwitchInfo {
            subject_raw: subject,
            arms,
            default_body: default_body.as_ref(),
            default_span: *default_span,
            mode: *mode,
            nocase: *nocase,
        },
        env,
    );
    for arm in arms {
        if let Some(body) = &arm.body {
            walk_script(ctx, body, env);
        }
    }
    if let Some(body) = default_body {
        walk_script(ctx, body, env);
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
    let octal = ctx.dialect.map(leading_zero_is_octal);
    for clause in clauses {
        let Some(val) = eval_tcl_expr_with_octal(&clause.condition, env, octal) else {
            return;
        };
        if val.is_truthy() {
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

/// Bundle of per-switch fields passed through static-match evaluation.
struct SwitchInfo<'a> {
    subject_raw: &'a str,
    arms: &'a [SwitchArm],
    default_body: Option<&'a Script>,
    default_span: Option<tcl_lexer::Span>,
    mode: SwitchMode,
    nocase: bool,
}

fn try_eliminate_switch(
    ctx: &mut PassContext<'_>,
    stmt_span: tcl_lexer::Span,
    info: &SwitchInfo<'_>,
    env: &Env,
) {
    // regexp mode is too complex to evaluate statically.
    if matches!(info.mode, SwitchMode::Regexp) {
        return;
    }
    let Some(subject) = resolve_subject(info.subject_raw, env) else {
        return;
    };

    for (idx, arm) in info.arms.iter().enumerate() {
        if !pattern_matches(&subject, &arm.pattern, info.mode, info.nocase) {
            continue;
        }
        // Walk past fall-through arms.
        let mut chosen_body: Option<&Script> = arm.body.as_ref();
        let mut chosen_span: Option<tcl_lexer::Span> = arm.body_span;
        if arm.fallthrough {
            let mut next = None;
            for subsequent in &info.arms[idx + 1..] {
                if !subsequent.fallthrough {
                    next = Some((subsequent.body.as_ref(), subsequent.body_span));
                    break;
                }
            }
            if let Some((body, span)) = next {
                chosen_body = body;
                chosen_span = span;
            } else {
                chosen_body = info.default_body;
                chosen_span = info.default_span;
            }
        }
        if let (Some(_body), Some(span)) = (chosen_body, chosen_span) {
            let replacement = extract_body_text(ctx.source, span, stmt_span);
            ctx.report(Optimisation::new(
                DiagCode::O112,
                format!(
                    "Eliminate switch (subject '{subject}' always matches pattern '{}')",
                    arm.pattern,
                ),
                full_rewrite_span(ctx.source, stmt_span),
                replacement,
            ));
        }
        return;
    }
    // No arm matched.
    if let (Some(_body), Some(span)) = (info.default_body, info.default_span) {
        let replacement = extract_body_text(ctx.source, span, stmt_span);
        ctx.report(Optimisation::new(
            DiagCode::O112,
            format!("Eliminate switch (subject '{subject}' matches no pattern); keep default"),
            full_rewrite_span(ctx.source, stmt_span),
            replacement,
        ));
    } else {
        ctx.report(Optimisation::new(
            DiagCode::O112,
            format!("Eliminate dead switch (subject '{subject}' matches no pattern)"),
            full_rewrite_span(ctx.source, stmt_span),
            "",
        ));
    }
}

/// Resolve a switch subject to a literal string. Returns `None`
/// when the subject contains substitutions we cannot fold.
fn resolve_subject(subject_raw: &str, env: &Env) -> Option<String> {
    if is_plain_literal(subject_raw) {
        return Some(subject_raw.to_owned());
    }
    // `$var` or `${var}` — try the SCCP env.
    let name = if let Some(inner) = subject_raw
        .strip_prefix("${")
        .and_then(|s| s.strip_suffix('}'))
    {
        inner.to_owned()
    } else {
        let rest = subject_raw.strip_prefix('$')?;
        rest.to_owned()
    };
    let var_ref = format!("${name}");
    let normalised = normalise_var_name(&var_ref).to_owned();
    let value = env.get(&name).or_else(|| env.get(&normalised))?;
    match value {
        EnvValue::Int(i) => Some(i.to_string()),
        EnvValue::Float(f) => Some(f.to_string()),
        EnvValue::Str(s) => Some(s.clone()),
    }
}

/// Match a `switch` arm pattern against `subject`, honouring
/// `mode` and `nocase`. `regexp` mode is rejected at the caller;
/// this helper handles `exact` and `glob`.
fn pattern_matches(subject: &str, pattern: &str, mode: SwitchMode, nocase: bool) -> bool {
    match mode {
        SwitchMode::Exact => {
            if nocase {
                subject.eq_ignore_ascii_case(pattern)
            } else {
                subject == pattern
            }
        }
        // `glob` folds through the shared `string match` dialect
        // (`tcl_syntax::glob`) so the optimiser and runtime agree.
        SwitchMode::Glob => tcl_syntax::glob::string_case_match(pattern, subject, nocase),
        SwitchMode::Regexp => false,
    }
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

    // internal helpers

    #[test]
    fn pattern_matches_exact_nocase_and_glob() {
        assert!(pattern_matches("Foo", "foo", SwitchMode::Exact, true));
        assert!(!pattern_matches("Foo", "foo", SwitchMode::Exact, false));
        assert!(pattern_matches("foo_bar", "foo_*", SwitchMode::Glob, false));
        assert!(!pattern_matches("foo", "bar", SwitchMode::Regexp, false));
    }

    #[test]
    fn resolve_subject_plain_literal_passes_through() {
        let env = Env::new();
        assert_eq!(resolve_subject("abc", &env).as_deref(), Some("abc"));
    }

    #[test]
    fn resolve_subject_dollar_var_looks_up_env() {
        let mut env = Env::new();
        env.insert("x".into(), EnvValue::Str("hi".into()));
        assert_eq!(resolve_subject("$x", &env).as_deref(), Some("hi"));
        assert_eq!(resolve_subject("${x}", &env).as_deref(), Some("hi"));
        // Missing binding → None.
        assert!(resolve_subject("$missing", &env).is_none());
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

    #[test]
    fn switch_regexp_mode_is_skipped() {
        // `switch -regexp foo { ... }` must not emit O112 even
        // when the subject is literal.
        let opts = run_pass("switch -regexp foo { ^foo$ { puts ok } }");
        assert!(
            opts.iter().all(|o| o.code != DiagCode::O112),
            "regexp switch must be skipped, got {opts:?}",
        );
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
        let cu = CompilationUnit::build_for("set x 7\nif {$x} { ok }", &registry(), false);
        let env = sccp_env_for(&cu.top_level);
        // `x` is constant → present in env.
        assert!(env.contains_key("x"));
    }

    #[test]
    fn unknown_condition_produces_no_fold() {
        // `$x` has no lattice binding → eval returns None → no
        // O112 from the if. Branch-folding isn't run here.
        let opts = run_pass("if {$x} { ok } else { bad }");
        assert!(opts.iter().all(|o| o.code != DiagCode::O112));
    }
}
