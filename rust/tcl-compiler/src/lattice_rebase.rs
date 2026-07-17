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

//! Offset rebasing for a memoised [`FunctionUnit`] (slice 4 offset-invariance).
//!
//! The per-procedure lattice cache keys on a procedure's **body source** (not
//! its position), so a body that is unchanged but *shifted* (lines inserted
//! above it) is a cache hit.  The cached unit's CFG/SSA/SCCP spans are
//! absolute, though, so on such a hit we must shift every span by the
//! difference between the procedure's current definition offset and the offset
//! the cached unit was built at.  The result is then byte-identical to a
//! freshly-built unit at the new position.
//!
//! Only the span-carrying parts of a [`FunctionUnit`] are traversed (every
//! other lattice field — `types`/`taints`/`rendered_props`/`def_use`/
//! `memory_ssa`/SSA phis — is span-free): the CFG block statements +
//! terminators + loop-node spans + inlined-`eval` body spans, the SSA blocks'
//! cloned statements (read for positions by some emitters), and the SCCP
//! constant-branch spans.
//! `ExprNode` carries *relative* offsets anchored to a statement span we shift,
//! so it needs no rebasing — but the absolute `expr_base` / `condition_base`
//! anchors those offsets map through do.

use tcl_lexer::Span;

use crate::cfg::Terminator;
use crate::compilation_unit::FunctionUnit;
use crate::ir::{CommandTokens, Script, Statement};

/// Shift every absolute span in `fu` by `delta` bytes (signed).  A no-op for
/// `delta == 0`.
pub(crate) fn rebase_function_unit(fu: &mut FunctionUnit, delta: i64) {
    if delta == 0 {
        return;
    }
    for block in fu.cfg.blocks.values_mut() {
        for stmt in &mut block.statements {
            rebase_statement(stmt, delta);
        }
        if let Some(term) = &mut block.terminator {
            rebase_terminator(term, delta);
        }
    }
    for loop_node in fu.cfg.loop_nodes.values_mut() {
        shift(&mut loop_node.span, delta);
        rebase_statement(&mut loop_node.for_stmt, delta);
    }
    // The inlined-`eval {…}` body spans are absolute, span-carrying state too;
    // without shifting them a cache-hit, offset-rebased unit keeps stale
    // offsets for any consumer (error-region mapping / explorer views) — issue
    // 149.
    for span in &mut fu.cfg.inline_eval_spans {
        shift(span, delta);
    }
    // SSA holds its own clones of the IR statements; some emitters read spans
    // from them (`stmt.statement.span()`), so rebase those too.
    for block in fu.ssa.blocks.values_mut() {
        for ssa_stmt in &mut block.statements {
            rebase_statement(&mut ssa_stmt.statement, delta);
        }
    }
    for cb in &mut fu.sccp.constant_branches {
        shift_opt(&mut cb.span, delta);
    }
}

fn shift(span: &mut Span, delta: i64) {
    let start = (i64::from(span.start()) + delta).max(0);
    let end = (i64::from(span.end()) + delta).max(0);
    // `start`/`end` are clamped to `>= 0`; spans are `u32` offsets, so a
    // shifted offset past `u32::MAX` is degenerate and clamps to the max.
    let start = u32::try_from(start).unwrap_or(u32::MAX);
    let end = u32::try_from(end).unwrap_or(u32::MAX);
    *span = Span::new(start, end);
}

fn shift_opt(span: &mut Option<Span>, delta: i64) {
    if let Some(span) = span {
        shift(span, delta);
    }
}

/// Shift an absolute expression-text anchor (`expr_base` / `condition_base`)
/// — same clamping as [`shift`].
fn shift_base(base: &mut Option<u32>, delta: i64) {
    if let Some(b) = base {
        let shifted = (i64::from(*b) + delta).max(0);
        *b = u32::try_from(shifted).unwrap_or(u32::MAX);
    }
}

fn rebase_tokens(tokens: &mut Option<CommandTokens>, delta: i64) {
    if let Some(tokens) = tokens {
        for span in &mut tokens.argv {
            shift(span, delta);
        }
        for span in &mut tokens.all_tokens {
            shift(span, delta);
        }
    }
}

/// Shift every absolute span in `script`'s statements by `delta` bytes.  Used
/// to normalise a procedure body to offset 0 before interning it as a
/// salsa-native [`crate::compilation_unit`] lattice key (and reused for nested
/// `Try` bodies during unit rebasing).
pub fn rebase_script(script: &mut Script, delta: i64) {
    if delta == 0 {
        return;
    }
    for stmt in &mut script.statements {
        rebase_statement(stmt, delta);
    }
}

fn rebase_terminator(term: &mut Terminator, delta: i64) {
    match term {
        Terminator::Goto { span, .. } | Terminator::Return { span, .. } => shift_opt(span, delta),
        Terminator::Branch {
            span,
            condition_base,
            ..
        } => {
            shift_opt(span, delta);
            shift_base(condition_base, delta);
        }
    }
}

fn rebase_statement(stmt: &mut Statement, delta: i64) {
    match stmt {
        Statement::AssignConst {
            span, value_span, ..
        } => {
            shift(span, delta);
            shift_opt(value_span, delta);
        }
        Statement::Incr { span, .. } | Statement::Return { span, .. } => shift(span, delta),
        Statement::AssignExpr {
            span, expr_base, ..
        }
        | Statement::ExprEval {
            span, expr_base, ..
        } => {
            shift(span, delta);
            shift_base(expr_base, delta);
        }
        Statement::AssignValue { span, tokens, .. }
        | Statement::Call { span, tokens, .. }
        | Statement::Barrier { span, tokens, .. } => {
            shift(span, delta);
            rebase_tokens(tokens, delta);
        }
        Statement::Block {
            span, body, tokens, ..
        }
        | Statement::UpFrame {
            span, body, tokens, ..
        } => {
            shift(span, delta);
            rebase_script(body, delta);
            rebase_tokens(tokens, delta);
        }
        Statement::If { .. } | Statement::Try { .. } | Statement::Switch { .. } => {
            rebase_branching_statement(stmt, delta);
        }
        Statement::For {
            span,
            init,
            init_span,
            next,
            next_span,
            condition_span,
            condition_base,
            body,
            body_span,
            raw_tokens,
            ..
        } => {
            shift(span, delta);
            shift(init_span, delta);
            shift(condition_span, delta);
            shift_base(condition_base, delta);
            shift(next_span, delta);
            shift(body_span, delta);
            rebase_script(init, delta);
            rebase_script(next, delta);
            rebase_script(body, delta);
            rebase_tokens(raw_tokens, delta);
        }
        Statement::While {
            span,
            condition_span,
            condition_base,
            body,
            body_span,
            raw_tokens,
            ..
        } => {
            shift(span, delta);
            shift(condition_span, delta);
            shift_base(condition_base, delta);
            shift(body_span, delta);
            rebase_script(body, delta);
            rebase_tokens(raw_tokens, delta);
        }
        Statement::Foreach {
            span,
            body,
            body_span,
            raw_tokens,
            ..
        } => {
            shift(span, delta);
            shift(body_span, delta);
            rebase_script(body, delta);
            rebase_tokens(raw_tokens, delta);
        }
        Statement::Catch {
            span,
            body,
            body_span,
            tokens,
            ..
        } => {
            shift(span, delta);
            shift(body_span, delta);
            rebase_script(body, delta);
            rebase_tokens(tokens, delta);
        }
    }
}

/// Rebase the span-bearing branching statements (`If` / `Try` / `Switch`),
/// extracted from [`rebase_statement`] to keep each function small.
fn rebase_branching_statement(stmt: &mut Statement, delta: i64) {
    match stmt {
        Statement::If {
            span,
            clauses,
            else_body,
            else_span,
        } => {
            shift(span, delta);
            shift_opt(else_span, delta);
            for clause in clauses {
                shift(&mut clause.condition_span, delta);
                shift_base(&mut clause.condition_base, delta);
                shift(&mut clause.body_span, delta);
                rebase_script(&mut clause.body, delta);
            }
            if let Some(body) = else_body {
                rebase_script(body, delta);
            }
        }
        Statement::Try {
            span,
            body,
            body_span,
            handlers,
            finally_body,
            finally_span,
            ..
        } => {
            shift(span, delta);
            shift(body_span, delta);
            shift_opt(finally_span, delta);
            rebase_script(body, delta);
            for handler in handlers {
                shift(&mut handler.body_span, delta);
                rebase_script(&mut handler.body, delta);
            }
            if let Some(finally) = finally_body {
                rebase_script(finally, delta);
            }
        }
        Statement::Switch {
            span,
            subject_span,
            arms,
            default_body,
            default_span,
            ..
        } => {
            shift(span, delta);
            shift(subject_span, delta);
            shift_opt(default_span, delta);
            for arm in arms {
                shift(&mut arm.pattern_span, delta);
                shift_opt(&mut arm.body_span, delta);
                if let Some(body) = &mut arm.body {
                    rebase_script(body, delta);
                }
            }
            if let Some(body) = default_body {
                rebase_script(body, delta);
            }
        }
        _ => unreachable!("rebase_branching_statement called on non-branching statement"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    /// A cache-hit, offset-rebased unit must shift its inlined-`eval` body
    /// spans along with every other absolute span, or error-region / explorer
    /// consumers see stale offsets (issue 149).
    #[test]
    fn rebase_shifts_inline_eval_spans() {
        let reg = CommandRegistry::build_default();
        // A static `eval {…}` body is flattened inline and its command span
        // recorded in `inline_eval_spans`.
        let cu = CompilationUnit::build_for("proc f {} { eval { set x 1 } }", &reg, false);
        let mut fu = cu.function("::f").expect("::f built").clone();
        assert!(
            !fu.cfg.inline_eval_spans.is_empty(),
            "an inlined eval body should record a span",
        );
        let before: Vec<Span> = fu.cfg.inline_eval_spans.clone();
        rebase_function_unit(&mut fu, 100);
        for (b, a) in before.iter().zip(&fu.cfg.inline_eval_spans) {
            assert_eq!(a.start(), b.start() + 100);
            assert_eq!(a.end(), b.end() + 100);
        }
    }
}
