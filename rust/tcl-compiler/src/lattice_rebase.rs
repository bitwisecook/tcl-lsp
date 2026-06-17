//! Offset normalisation for a memoised procedure body (slice 4 / Approach B).
//!
//! The per-procedure lattice cache keys on a procedure's **body source** (not
//! its position), so a body that is unchanged but *shifted* (lines inserted
//! above it) is a cache hit.  To form that position-independent key the body is
//! normalised to **offset 0** with [`rebase_script`] (every span shifted by
//! `-body_offset`).
//!
//! The reverse — shifting a whole built unit back to its real position — is no
//! longer done: under Approach B the diagnostic consumers consume the offset-0
//! unit plus its [`crate::compilation_unit::FunctionUnit::base_offset`] and add
//! the offset at emit time (`abs_span`), so the O(unit) span walk is gone.
//! `ExprNode` carries *relative* offsets anchored to a statement span, so it
//! needs no rebasing.

use tcl_lexer::Span;

use crate::ir::{CommandTokens, Script, Statement};

fn shift(span: &mut Span, delta: i64) {
    let start = (i64::from(span.start()) + delta).max(0);
    let end = (i64::from(span.end()) + delta).max(0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        *span = Span::new(start as u32, end as u32);
    }
}

fn shift_opt(span: &mut Option<Span>, delta: i64) {
    if let Some(span) = span {
        shift(span, delta);
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
pub(crate) fn rebase_script(script: &mut Script, delta: i64) {
    if delta == 0 {
        return;
    }
    for stmt in &mut script.statements {
        rebase_statement(stmt, delta);
    }
}

#[allow(clippy::too_many_lines)]
fn rebase_statement(stmt: &mut Statement, delta: i64) {
    match stmt {
        Statement::AssignConst { span, .. }
        | Statement::AssignExpr { span, .. }
        | Statement::Incr { span, .. }
        | Statement::ExprEval { span, .. }
        | Statement::Return { span, .. } => shift(span, delta),
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
                shift(&mut clause.body_span, delta);
                rebase_script(&mut clause.body, delta);
            }
            if let Some(body) = else_body {
                rebase_script(body, delta);
            }
        }
        Statement::For {
            span,
            init,
            init_span,
            next,
            next_span,
            condition_span,
            body,
            body_span,
            ..
        } => {
            shift(span, delta);
            shift(init_span, delta);
            shift(condition_span, delta);
            shift(next_span, delta);
            shift(body_span, delta);
            rebase_script(init, delta);
            rebase_script(next, delta);
            rebase_script(body, delta);
        }
        Statement::While {
            span,
            condition_span,
            body,
            body_span,
            ..
        } => {
            shift(span, delta);
            shift(condition_span, delta);
            shift(body_span, delta);
            rebase_script(body, delta);
        }
        Statement::Foreach {
            span,
            body,
            body_span,
            ..
        } => {
            shift(span, delta);
            shift(body_span, delta);
            rebase_script(body, delta);
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
    }
}
