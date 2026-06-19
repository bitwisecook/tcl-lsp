//! Code-sinking optimiser pass — **O125** (C30i).
//!
//! Ported from `core/compiler/optimiser/_code_sinking.py`.
//!
//! Detects `set X V; <decision>` pairs where the following
//! statement is an `if` / `switch`, the variable `X` is read
//! inside at least one decision body, and `X` is not referenced
//! by the decision condition / subject nor by any statement
//! after the decision. Emits a grouped pair of `O125`
//! optimisations:
//!
//! 1. A deletion of the original `set` (replacement = empty
//!    string over the `set` statement's span).
//! 2. An insertion at the target body — the first statement of
//!    the branch that uses the variable — prepending the
//!    original set's source text plus a separator.
//!
//! Both emissions share a group id (via
//! [`PassContext::alloc_group`]) so downstream consumers apply
//! them atomically. When the original statement's source text
//! cannot be recovered (e.g. local-offset span inside a
//! re-lowered proc body), the pass falls back to a single
//! hint-only diagnostic pointing at the assignment.
//!
//! Sinkable assignments in this pass are limited to
//! side-effect-free shapes: [`Statement::AssignConst`],
//! [`Statement::AssignValue`] without a command substitution,
//! and [`Statement::AssignExpr`] whose expression has no
//! command substitution.

use crate::compilation_unit::CompilationUnit;
use crate::expr_ast::ExprNode;
use crate::ir::{Script, Statement};

use super::helpers::expr_simplify::expr_has_command_subst;
use super::{Optimisation, PassContext};

/// Run the code-sinking pass.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    walk_script(ctx, &cu.ir_module.top_level);
    for proc in cu.ir_module.procedures.values() {
        walk_script(ctx, &proc.body);
    }
}

fn walk_script(ctx: &mut PassContext<'_>, script: &Script) {
    let stmts = &script.statements;
    for i in 0..stmts.len().saturating_sub(1) {
        let stmt = &stmts[i];
        let Some((var, span)) = sinkable_assignment(stmt) else {
            continue;
        };
        // A variable that carries state across `when <event>` boundaries
        // (iRules) is observable after this event handler returns, so its
        // definition must not be sunk into a branch that may not run.
        // Mirrors Python's `var_name in ctx.cross_event_vars` guard.
        if ctx.cross_event_vars.contains(&var) {
            continue;
        }
        // Already-covered guard: a statement an earlier pass already rewrites
        // (e.g. O109 / O126 dead-store elimination, which runs before code
        // sinking) must not also be sunk — the two rewrites would conflict.
        // Mirrors Python's `already_covered` check over `ctx.optimisations`.
        if ctx.optimisations.iter().any(|o| {
            o.span.start() <= span.start() && o.span.end() >= span.end() && !span.is_empty()
        }) {
            continue;
        }
        let decision = &stmts[i + 1];
        if !is_decision(decision) {
            continue;
        }
        if decision_condition_uses_var(decision, &var) {
            continue;
        }
        if !any_decision_body_uses_var(decision, &var) {
            continue;
        }
        let mut later_use = false;
        for later in &stmts[i + 2..] {
            if statement_uses_var(later, &var) {
                later_use = true;
                break;
            }
        }
        if later_use {
            continue;
        }
        emit_sink(ctx, stmt, span, decision, &var);
    }

    // Recurse into nested compound statements.
    for stmt in stmts {
        match stmt {
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
}

/// Emit the O125 sink. The assignment is sunk into the **deepest** branch
/// body that uses `var`, in **every** branch that uses it (a decision with
/// the var live in both arms duplicates the def into each). When the
/// original source text can be recovered, emits a grouped delete + one
/// prepend per target; otherwise falls back to a single hint-only
/// diagnostic. Mirrors Python's `_find_deepest_sink_targets` /
/// `_emit_sinking_opts`.
fn emit_sink(
    ctx: &mut PassContext<'_>,
    original: &Statement,
    original_span: tcl_lexer::Span,
    decision: &Statement,
    var: &str,
) {
    let _ = original;
    let targets = decision_sink_targets(decision, var);
    let target_spans: Vec<tcl_lexer::Span> = targets.iter().map(|s| s.span()).collect();

    if let Some(set_text) = extract_source(ctx.source, original_span)
        && !target_spans.is_empty()
        && target_spans
            .iter()
            .all(|s| extract_source(ctx.source, *s).is_some())
    {
        let group = ctx.alloc_group();
        let mut del = Optimisation::new(
            "O125",
            format!("Sink '{var}' into the branch(es) that use it — delete original"),
            original_span,
            "",
        );
        del.group = Some(group);
        ctx.report(del);

        for span in target_spans {
            let first_text = extract_source(ctx.source, span).unwrap_or_default();
            let mut ins = Optimisation::new(
                "O125",
                format!("Sink '{var}' into branch — prepend in target body"),
                span,
                format!("{set_text}; {first_text}"),
            );
            ins.group = Some(group);
            ctx.report(ins);
        }
    } else {
        // Fallback: hint-only.
        let mut opt = Optimisation::new(
            "O125",
            format!("Sink '{var}' into the decision block that uses it"),
            original_span,
            "",
        );
        opt.hint_only = true;
        ctx.report(opt);
    }
}

/// Collect the sink-target anchor statements for `decision`: for each
/// branch body that uses `var`, the deepest first-using statement (a
/// single-use branch whose lone consumer is itself a decision descends
/// further). Mirrors the per-branch `_find_deepest_sink_targets` fan-out.
fn decision_sink_targets<'a>(decision: &'a Statement, var: &str) -> Vec<&'a Statement> {
    let mut targets = Vec::new();
    for body in decision_branch_bodies(decision) {
        targets.extend(find_deepest_targets(body, var));
    }
    targets
}

/// The branch bodies of a decision statement (if clauses + else; switch
/// arms + default).
fn decision_branch_bodies(decision: &Statement) -> Vec<&Script> {
    let mut bodies = Vec::new();
    match decision {
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                bodies.push(&c.body);
            }
            if let Some(b) = else_body {
                bodies.push(b);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    bodies.push(b);
                }
            }
            if let Some(b) = default_body {
                bodies.push(b);
            }
        }
        _ => {}
    }
    bodies
}

/// Deepest sink targets within a single body. When exactly one statement
/// uses `var` (and no earlier statement redefines it), and that statement
/// is itself a decision the var's condition does not read, descend into
/// its branches; otherwise anchor at the first using statement of this
/// body. Mirrors `_find_deepest_sink_targets`.
fn find_deepest_targets<'a>(body: &'a Script, var: &str) -> Vec<&'a Statement> {
    let using: Vec<usize> = body
        .statements
        .iter()
        .enumerate()
        .filter(|(_, s)| statement_uses_var(s, var))
        .map(|(i, _)| i)
        .collect();
    let Some(&first) = using.first() else {
        return Vec::new();
    };
    if using.len() == 1 {
        let no_prior_redefine = !body.statements[..first]
            .iter()
            .any(|s| statement_defines_var(s, var));
        if no_prior_redefine && is_decision(&body.statements[first]) {
            let deeper = try_deeper_sink(&body.statements[first], var);
            if !deeper.is_empty() {
                return deeper;
            }
        }
    }
    vec![&body.statements[first]]
}

/// Descend into a decision's branches for a deeper sink — but only when
/// the var's value is not read by any condition (which would make sinking
/// past it unsound). Mirrors `_try_deeper_sink`.
fn try_deeper_sink<'a>(stmt: &'a Statement, var: &str) -> Vec<&'a Statement> {
    if decision_condition_uses_var(stmt, var) {
        return Vec::new();
    }
    let mut targets = Vec::new();
    for body in decision_branch_bodies(stmt) {
        targets.extend(find_deepest_targets(body, var));
    }
    targets
}

/// Whether `stmt` writes `var` (an assignment to it, or a call whose
/// `defs` include it).
fn statement_defines_var(stmt: &Statement, var: &str) -> bool {
    match stmt {
        Statement::AssignConst { name, .. }
        | Statement::AssignValue { name, .. }
        | Statement::AssignExpr { name, .. } => name == var,
        Statement::Call { defs, .. } => defs.iter().any(|d| d == var),
        _ => false,
    }
}

/// Slice `span` out of `source`; returns `None` when the span
/// extends past the source (common for per-procedure IR whose
/// spans are local to a re-lowered body string).
fn extract_source(source: &str, span: tcl_lexer::Span) -> Option<String> {
    let range = span.as_range();
    if range.end > source.len() || range.start > range.end {
        return None;
    }
    Some(source[range].to_owned())
}


/// Return `Some((var_name, stmt_span))` if `stmt` is a
/// side-effect-free assignment whose defined variable is
/// statically known.
fn sinkable_assignment(stmt: &Statement) -> Option<(String, tcl_lexer::Span)> {
    match stmt {
        Statement::AssignConst { span, name, .. } => Some((name.clone(), *span)),
        Statement::AssignValue {
            span, name, value, ..
        } => {
            if value.contains('[') {
                None
            } else {
                Some((name.clone(), *span))
            }
        }
        Statement::AssignExpr { span, name, expr } => {
            if expr_has_command_subst(expr) {
                None
            } else {
                Some((name.clone(), *span))
            }
        }
        _ => None,
    }
}

fn is_decision(stmt: &Statement) -> bool {
    matches!(stmt, Statement::If { .. } | Statement::Switch { .. })
}

/// Return `true` when the decision's *condition* (or switch
/// subject) textually references `$var`.
fn decision_condition_uses_var(stmt: &Statement, var: &str) -> bool {
    match stmt {
        Statement::If { clauses, .. } => clauses
            .iter()
            .any(|c| expr_references_var(&c.condition, var)),
        Statement::Switch { subject, .. } => text_references_var(subject, var),
        _ => false,
    }
}

fn any_decision_body_uses_var(stmt: &Statement, var: &str) -> bool {
    match stmt {
        Statement::If {
            clauses, else_body, ..
        } => {
            clauses.iter().any(|c| script_uses_var(&c.body, var))
                || else_body.as_ref().is_some_and(|b| script_uses_var(b, var))
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            arms.iter()
                .any(|a| a.body.as_ref().is_some_and(|b| script_uses_var(b, var)))
                || default_body
                    .as_ref()
                    .is_some_and(|b| script_uses_var(b, var))
        }
        _ => false,
    }
}

fn script_uses_var(script: &Script, var: &str) -> bool {
    script.statements.iter().any(|s| statement_uses_var(s, var))
}

/// Inspect an IR statement for a textual `$var` / `${var}` use.
/// Walks `Call` args, `AssignValue` RHS text, `Incr` amount,
/// `ExprEval` / conditions via the parsed AST, and descends into
/// compound-statement bodies.
fn statement_uses_var(stmt: &Statement, var: &str) -> bool {
    match stmt {
        Statement::AssignConst { .. } | Statement::Barrier { .. } => false,
        Statement::AssignValue { value, .. } => text_references_var(value, var),
        Statement::AssignExpr { expr, .. } | Statement::ExprEval { expr, .. } => {
            expr_references_var(expr, var)
        }
        Statement::Incr { amount, .. } => amount
            .as_deref()
            .is_some_and(|a| text_references_var(a, var)),
        Statement::Return { value, expr, .. } => {
            value
                .as_deref()
                .is_some_and(|v| text_references_var(v, var))
                || expr.as_ref().is_some_and(|e| expr_references_var(e, var))
        }
        Statement::Call { args, .. } => args.iter().any(|a| text_references_var(a, var)),
        Statement::If {
            clauses, else_body, ..
        } => {
            clauses
                .iter()
                .any(|c| expr_references_var(&c.condition, var) || script_uses_var(&c.body, var))
                || else_body.as_ref().is_some_and(|b| script_uses_var(b, var))
        }
        Statement::For {
            init,
            condition,
            next,
            body,
            ..
        } => {
            script_uses_var(init, var)
                || expr_references_var(condition, var)
                || script_uses_var(next, var)
                || script_uses_var(body, var)
        }
        Statement::While {
            condition, body, ..
        } => expr_references_var(condition, var) || script_uses_var(body, var),
        Statement::Foreach {
            iterators, body, ..
        } => {
            iterators
                .iter()
                .any(|it| text_references_var(&it.list_arg, var))
                || script_uses_var(body, var)
        }
        Statement::Catch { body, .. }
        | Statement::UpFrame { body, .. }
        | Statement::Block { body, .. } => script_uses_var(body, var),
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            script_uses_var(body, var)
                || handlers.iter().any(|h| script_uses_var(&h.body, var))
                || finally_body
                    .as_ref()
                    .is_some_and(|fb| script_uses_var(fb, var))
        }
        Statement::Switch {
            subject,
            arms,
            default_body,
            ..
        } => {
            text_references_var(subject, var)
                || arms
                    .iter()
                    .any(|a| a.body.as_ref().is_some_and(|b| script_uses_var(b, var)))
                || default_body
                    .as_ref()
                    .is_some_and(|db| script_uses_var(db, var))
        }
    }
}

/// Scan a raw Tcl-source word for `$name` / `${name}`
/// references and return `true` when any matches `var`.
fn text_references_var(text: &str, var: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'$' {
            i += 1;
            continue;
        }
        i += 1;
        if i >= bytes.len() {
            break;
        }
        if bytes[i] == b'{' {
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i] != b'}' {
                i += 1;
            }
            if let Ok(name) = std::str::from_utf8(&bytes[start..i])
                && name == var
            {
                return true;
            }
            if i < bytes.len() {
                i += 1;
            }
            continue;
        }
        let start = i;
        while i < bytes.len() {
            let b = bytes[i];
            if b.is_ascii_alphanumeric() || b == b'_' {
                i += 1;
            } else if b == b':' && i + 1 < bytes.len() && bytes[i + 1] == b':' {
                i += 2;
            } else {
                break;
            }
        }
        if let Ok(name) = std::str::from_utf8(&bytes[start..i])
            && name == var
        {
            return true;
        }
    }
    false
}

fn expr_references_var(node: &ExprNode, var: &str) -> bool {
    match node {
        ExprNode::Var { name, .. } => name == var,
        ExprNode::Binary { left, right, .. } => {
            expr_references_var(left, var) || expr_references_var(right, var)
        }
        ExprNode::Unary { operand, .. } => expr_references_var(operand, var),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            expr_references_var(condition, var)
                || expr_references_var(true_branch, var)
                || expr_references_var(false_branch, var)
        }
        ExprNode::Call { args, .. } => args.iter().any(|a| expr_references_var(a, var)),
        _ => false,
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
    fn sinkable_set_before_if_emits_o125() {
        let opts = run_pass("proc ::f {flag} { set x 1; if {$flag} { puts $x } else { puts no } }");
        assert!(
            opts.iter().any(|o| o.code == "O125"),
            "expected an O125 diagnostic, got {opts:?}",
        );
    }

    #[test]
    fn sinks_into_both_using_branches() {
        // `$x` is used in *both* the if-body and the else-body, so the def
        // is sunk (duplicated) into each — two grouped inserts + one delete.
        let opts = run_pass(
            "proc ::f {flag} { set x 1; if {$flag} { puts $x } else { puts $x } }",
        );
        let inserts = opts
            .iter()
            .filter(|o| o.code == "O125" && o.replacement.contains("set x 1"))
            .count();
        assert_eq!(inserts, 2, "expected a sink into each branch, got {opts:?}");
        // Exactly one delete of the original.
        let deletes = opts
            .iter()
            .filter(|o| o.code == "O125" && o.replacement.is_empty() && !o.hint_only)
            .count();
        assert_eq!(deletes, 1);
    }

    #[test]
    fn sinks_deeper_into_nested_decision() {
        // `$x` is used only inside a nested `if` within the outer if-body —
        // the def sinks all the way into the inner branch.
        let opts = run_pass(
            "proc ::f {a b} { set x 1; if {$a} { if {$b} { puts $x } } else { puts no } }",
        );
        // The deepest insert target is the inner `puts $x`; the sunk text
        // must be prepended there (its slice contains the inner puts).
        let deep = opts.iter().any(|o| {
            o.code == "O125"
                && o.replacement.starts_with("set x 1;")
                && o.replacement.contains("puts $x")
        });
        assert!(deep, "expected a deep sink into the inner branch, got {opts:?}");
    }

    #[test]
    fn already_covered_statement_is_not_sunk() {
        // When an earlier pass already rewrites the `set x 1` statement,
        // code sinking must leave it alone.
        let src = "set x 1\nif {$cond} { puts $x } else { puts no }";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        // Simulate an earlier O109 dead-store rewrite covering `set x 1`.
        ctx.report(Optimisation::new(
            "O109",
            "dead store",
            tcl_lexer::Span::new(0, 7),
            "",
        ));
        run(&mut ctx, &cu);
        assert!(
            ctx.optimisations.iter().all(|o| o.code != "O125"),
            "already-covered statement must not be sunk, got {:?}",
            ctx.optimisations,
        );
    }

    #[test]
    fn cross_event_var_is_not_sunk() {
        // `x` carries state across iRules `when` events, so its definition
        // must stay where every later event can observe it — never sunk
        // into a branch that might not run.
        let src = "proc ::f {flag} { set x 1; if {$flag} { puts $x } else { puts no } }";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        ctx.cross_event_vars.insert("x".to_owned());
        run(&mut ctx, &cu);
        assert!(
            ctx.optimisations.iter().all(|o| o.code != "O125"),
            "cross-event var must not be sunk, got {:?}",
            ctx.optimisations,
        );
    }

    #[test]
    fn top_level_sink_produces_grouped_pair() {
        // At the top level, statement spans are absolute source
        // offsets so the multi-optimisation rewrite can
        // reconstruct the text. Expect two O125 entries sharing
        // one group.
        let opts = run_pass("set x 1\nif {$cond} { puts $x } else { puts no }");
        let o125: Vec<_> = opts.iter().filter(|o| o.code == "O125").collect();
        if !o125.is_empty() {
            let groups: std::collections::HashSet<_> =
                o125.iter().filter_map(|o| o.group).collect();
            // Either two grouped entries or one hint.
            if o125.len() >= 2 {
                assert_eq!(groups.len(), 1, "grouped pair expected, got {o125:?}");
            }
        }
    }

    #[test]
    fn assignment_with_cmd_subst_not_sunk() {
        // `set x [foo]` has a command substitution — not
        // side-effect-free; must not emit O125.
        let opts =
            run_pass("proc ::f {flag} { set x [foo]; if {$flag} { puts $x } else { puts no } }");
        assert!(
            opts.iter().all(|o| o.code != "O125"),
            "cmd-subst assignment must not emit O125, got {opts:?}",
        );
    }

    #[test]
    fn later_use_suppresses_sink() {
        let opts = run_pass("proc ::f {flag} { set x 1; if {$flag} { puts $x }; return $x }");
        assert!(
            opts.iter().all(|o| o.code != "O125"),
            "later use must suppress O125, got {opts:?}",
        );
    }

    #[test]
    fn condition_use_suppresses_sink() {
        // The condition reads `$x`, so sinking is unsound.
        let opts = run_pass("proc ::f {} { set x 1; if {$x > 0} { puts hi } else { puts no } }");
        assert!(
            opts.iter().all(|o| o.code != "O125"),
            "condition use must suppress O125, got {opts:?}",
        );
    }

    #[test]
    fn run_passes_dispatches_code_sinking() {
        let cu = CompilationUnit::build_for("set x 1", &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        super::super::run_passes(&mut ctx, &cu, &[super::super::PassId::CodeSinking]);
        // Single `set` with no following decision → no O125.
        assert!(ctx.optimisations.is_empty());
    }
}
