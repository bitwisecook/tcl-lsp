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

//! Which procedure bodies may be spliced: namespace invariance, splice-eligible statements and the parameterised (v3) rules.

use super::{InliningSummaries, MAX_INLINING_WALK_DEPTH, frame};
use tcl_compiler::ir::{CommandTokens, Procedure, Script, Statement};

// splice-eligibility

/// True when `command` resolves the same way from any caller's namespace
/// as it does from the callee's: it is fully qualified, or it is
/// unqualified and doesn't resolve to any known proc (so it's a global
/// builtin).
fn command_is_namespace_invariant(
    command: &str,
    _callee_qname: &str,
    tokens: &CommandTokens,
    summaries: &InliningSummaries<'_>,
) -> bool {
    if tcl_syntax::naming::is_absolute(command) {
        return true;
    }
    summaries.selected_body(tokens).is_none()
}

/// True iff `stmt` can be lifted out of the wrapper proc and spliced into
/// a caller without changing observable behaviour.
pub(super) fn stmt_is_splice_eligible(
    stmt: &Statement,
    callee_qname: &str,
    summaries: &InliningSummaries<'_>,
    source: frame::SourceContext<'_>,
) -> bool {
    let Statement::Call {
        command,
        defs,
        tokens: Some(tokens),
        ..
    } = stmt
    else {
        return false;
    };
    let Some(invocation) = source.invocation(tokens) else {
        return false;
    };
    defs.is_empty()
        && command_is_namespace_invariant(command, callee_qname, tokens, summaries)
        && tcl_registry::traits::is_splice_safe(invocation.facts.traits)
        && tcl_compiler::word_subst::checked_lifted_calls(tokens, source.config)
            .is_some_and(|calls| calls.is_empty())
}

// v3 eligibility

/// Return true iff `proc` qualifies for parameterised (v3) inlining.
pub(super) fn v3_eligible(
    proc: &Procedure,
    qname: &str,
    summaries: &InliningSummaries<'_>,
    source: frame::SourceContext<'_>,
) -> bool {
    if has_irreturn_in_unsafe_scope(&proc.body, false, 0) {
        return false;
    }
    for stmt in &proc.body.statements {
        if matches!(stmt, Statement::Return { .. }) {
            continue; // top-level return is fine — handled by wrap or kept
        }
        if !v3_stmt_eligible(stmt, qname, summaries, source) {
            return false;
        }
    }
    true
}

/// Walk `script` looking for any [`Statement::Return`] inside a construct
/// whose `break` semantics would trap our wrapper's break (loop bodies,
/// `catch` / `try` traps, `uplevel` frame-shifts). Transparent groupings
/// (`Block`, `if` clauses, `switch` arms) propagate the parent flag.
/// `depth` is the nesting level of `script` — see
/// [`MAX_INLINING_WALK_DEPTH`]. Past the cap, conservatively answers
/// `true` ("assume an unsafe-scope return exists") — this predicate gates
/// whether a proc is *safe* to v3-inline, so an unresolved deep answer
/// must lean toward declining the inline, never toward inlining something
/// that could trap a `break`.
pub(super) fn has_irreturn_in_unsafe_scope(
    script: &Script,
    inside_unsafe: bool,
    depth: u32,
) -> bool {
    if MAX_INLINING_WALK_DEPTH.exceeded(depth) {
        return true;
    }
    for stmt in &script.statements {
        match stmt {
            Statement::Return { .. } => {
                if inside_unsafe {
                    return true;
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                if has_irreturn_in_unsafe_scope(init, inside_unsafe, depth + 1)
                    || has_irreturn_in_unsafe_scope(next, inside_unsafe, depth + 1)
                    || has_irreturn_in_unsafe_scope(body, true, depth + 1)
                {
                    return true;
                }
            }
            Statement::While { body, .. }
            | Statement::Foreach { body, .. }
            | Statement::Catch { body, .. }
            | Statement::UpFrame { body, .. } => {
                if has_irreturn_in_unsafe_scope(body, true, depth + 1) {
                    return true;
                }
            }
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                if has_irreturn_in_unsafe_scope(body, true, depth + 1) {
                    return true;
                }
                for h in handlers {
                    if has_irreturn_in_unsafe_scope(&h.body, true, depth + 1) {
                        return true;
                    }
                }
                if let Some(fb) = finally_body
                    && has_irreturn_in_unsafe_scope(fb, true, depth + 1)
                {
                    return true;
                }
            }
            Statement::Block { body, .. } => {
                if has_irreturn_in_unsafe_scope(body, inside_unsafe, depth + 1) {
                    return true;
                }
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    if has_irreturn_in_unsafe_scope(&c.body, inside_unsafe, depth + 1) {
                        return true;
                    }
                }
                if let Some(b) = else_body
                    && has_irreturn_in_unsafe_scope(b, inside_unsafe, depth + 1)
                {
                    return true;
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for a in arms {
                    if let Some(b) = &a.body
                        && has_irreturn_in_unsafe_scope(b, inside_unsafe, depth + 1)
                    {
                        return true;
                    }
                }
                if let Some(b) = default_body
                    && has_irreturn_in_unsafe_scope(b, inside_unsafe, depth + 1)
                {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// Per-statement eligibility for v3 inlining.
fn v3_stmt_eligible(
    stmt: &Statement,
    callee_qname: &str,
    summaries: &InliningSummaries<'_>,
    source: frame::SourceContext<'_>,
) -> bool {
    match stmt {
        Statement::AssignConst { name, .. }
        | Statement::AssignValue { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::Incr { name, .. } => {
            // `::`-qualified names (globals) decline — the rename map only
            // covers locals. Array writes (`arr(idx)`) qualify.
            !tcl_syntax::naming::is_qualified(name.as_bytes())
        }
        Statement::Call { .. } => stmt_is_splice_eligible(stmt, callee_qname, summaries, source),
        Statement::ExprEval { .. } => true,
        Statement::If {
            clauses, else_body, ..
        } => {
            clauses
                .iter()
                .all(|c| v3_script_eligible(&c.body, callee_qname, summaries, source))
                && else_body
                    .as_ref()
                    .is_none_or(|b| v3_script_eligible(b, callee_qname, summaries, source))
        }
        Statement::For {
            init, next, body, ..
        } => {
            v3_script_eligible(init, callee_qname, summaries, source)
                && v3_script_eligible(next, callee_qname, summaries, source)
                && v3_script_eligible(body, callee_qname, summaries, source)
        }
        Statement::Block { body, .. }
        | Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. }
        | Statement::UpFrame { body, .. } => {
            v3_script_eligible(body, callee_qname, summaries, source)
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            v3_script_eligible(body, callee_qname, summaries, source)
                && handlers
                    .iter()
                    .all(|h| v3_script_eligible(&h.body, callee_qname, summaries, source))
                && finally_body
                    .as_ref()
                    .is_none_or(|b| v3_script_eligible(b, callee_qname, summaries, source))
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            arms.iter().all(|a| {
                a.body
                    .as_ref()
                    .is_none_or(|b| v3_script_eligible(b, callee_qname, summaries, source))
            }) && default_body
                .as_ref()
                .is_none_or(|b| v3_script_eligible(b, callee_qname, summaries, source))
        }
        // Return is handled by the caller; Barrier and anything else decline.
        _ => false,
    }
}

/// Recurse into a nested script and check per-statement eligibility.
/// `return` inside a nested script IS allowed (the for/break wrap routes
/// it).
fn v3_script_eligible(
    script: &Script,
    callee_qname: &str,
    summaries: &InliningSummaries<'_>,
    source: frame::SourceContext<'_>,
) -> bool {
    if tcl_compiler::native_compilation_admission::script_requires_admission(script) {
        return false;
    }
    for stmt in &script.statements {
        if matches!(stmt, Statement::Return { .. }) {
            continue;
        }
        if !v3_stmt_eligible(stmt, callee_qname, summaries, source) {
            return false;
        }
    }
    true
}
