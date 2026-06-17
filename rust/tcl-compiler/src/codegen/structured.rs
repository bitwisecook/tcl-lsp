//! The structured walk — the target-agnostic driver that walks the **structured
//! IR** ([`Statement`] tree) and drives a backend through the [`Emit`] seam.
//!
//! Unlike the bytecode emitter — which consumes the *flattened* CFG and emits
//! address jumps — a structured backend (WASM) needs nested regions, and the IR
//! already has them: `if`/`while`/`for` are nested [`Script`]s, so there is
//! nothing to reconstruct. The driver recurses the tree and emits structured
//! control flow directly:
//!
//! - `if`/`elseif`/`else` → nested [`Emit::begin_if`] / [`Emit::begin_else`].
//! - `while`/`for` → the [`Emit`] loop protocol; `break`/`continue`/`return`
//!   become the loop/function completion codes.
//! - everything whose control flow or iteration eval-fallback can't realise on
//!   its own (`foreach`/`switch`/`catch`/`try`, assignments, barriers) degrades
//!   to a single whole-construct eval-fallback of its source span.
//!
//! Walking the IR (rather than the CFG) keeps this correct by construction:
//! there are no back-edges to detect, no reconvergence joins to compute, and
//! `break`/`continue` target depth is just the recursion depth of enclosing
//! loops.

use tcl_lexer::Span;

use crate::codegen::emit::Emit;
use crate::ir::{IfClause, Script, Statement};

/// Whether straight-line control falls through to the next statement, or the
/// statement transferred control elsewhere (so the rest of its script is dead).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Flow {
    /// Control falls through to the following statement.
    Normal,
    /// `break` / `continue` / `return` was emitted; the rest of the enclosing
    /// straight-line script is unreachable and must not be emitted.
    Diverged,
}

/// Walk `script` (using `source` for eval-fallback command text), driving `emit`
/// with the recovered structured control flow.
pub fn walk<E: Emit>(emit: &mut E, script: &Script, source: &str) {
    walk_script(emit, script, source, 0);
}

/// Emit each statement of `script` in order, stopping after one diverges (its
/// successors are dead code). `loop_depth` is the number of structurally
/// enclosing loops whose break/continue scope is open — `break`/`continue` are
/// realised structurally only when inside one (`> 0`).
fn walk_script<E: Emit>(emit: &mut E, script: &Script, source: &str, loop_depth: u32) {
    for stmt in &script.statements {
        if walk_stmt(emit, stmt, source, loop_depth) == Flow::Diverged {
            break;
        }
    }
}

fn walk_stmt<E: Emit>(emit: &mut E, stmt: &Statement, source: &str, loop_depth: u32) -> Flow {
    match stmt {
        Statement::If {
            clauses,
            else_body,
            ..
        } => {
            emit_if(emit, clauses, else_body.as_ref(), source, loop_depth);
            Flow::Normal
        }

        Statement::While {
            condition_span,
            body,
            ..
        } => {
            let cond = condition_text(source, *condition_span);
            emit_loop(emit, opt(cond), body, None, source, loop_depth);
            Flow::Normal
        }

        Statement::For {
            init,
            condition_span,
            next_span,
            body,
            ..
        } => {
            // The init clause runs once, in the enclosing scope.
            walk_script(emit, init, source, loop_depth);
            let cond = condition_text(source, *condition_span);
            let step = condition_text(source, *next_span);
            emit_loop(emit, opt(cond), body, opt(step), source, loop_depth);
            Flow::Normal
        }

        Statement::Return { span, .. } => {
            // Eval the `return` command so the runtime sets the result/code,
            // then leave the function. (Eval-fallback discards the returned code
            // today; faithful propagation lands with the runtime ABI.)
            emit.emit_command(slice(source, *span));
            emit.emit_return();
            Flow::Diverged
        }

        Statement::Call { command, span, .. } => {
            // `break` / `continue` inside a loop are completion codes, realised
            // as structured jumps; everywhere else they are ordinary commands
            // (the runtime raises "invoked … outside of a loop").
            if loop_depth > 0 && command == "break" {
                emit.emit_break();
                Flow::Diverged
            } else if loop_depth > 0 && command == "continue" {
                emit.emit_continue();
                Flow::Diverged
            } else {
                emit.emit_command(slice(source, *span));
                Flow::Normal
            }
        }

        // Everything else — assignments, `foreach`/`switch`/`catch`/`try`,
        // barriers, inlined blocks — is one whole-construct eval-fallback.
        other => {
            emit.emit_command(slice(source, other.span()));
            Flow::Normal
        }
    }
}

/// Emit an `if`/`elseif`/`else` chain, desugaring `elseif` into nested
/// `if`/`else` so the backend sees only the two-armed primitive.
fn emit_if<E: Emit>(
    emit: &mut E,
    clauses: &[IfClause],
    else_body: Option<&Script>,
    source: &str,
    loop_depth: u32,
) {
    let Some((first, rest)) = clauses.split_first() else {
        return; // malformed `if` with no clauses — nothing to emit.
    };

    emit.begin_if(condition_text(source, first.condition_span));
    walk_script(emit, &first.body, source, loop_depth);

    if !rest.is_empty() {
        // Remaining `elseif` clauses become a nested `if` in the `else` arm.
        emit.begin_else();
        emit_if(emit, rest, else_body, source, loop_depth);
        emit.end_if();
    } else if let Some(eb) = else_body {
        emit.begin_else();
        walk_script(emit, eb, source, loop_depth);
        emit.end_if();
    } else {
        emit.end_if();
    }
}

/// Drive the [`Emit`] loop protocol for a `while`/`for`. `step` is the `for`
/// *next* clause (a single eval-fallback), absent for `while`.
fn emit_loop<E: Emit>(
    emit: &mut E,
    cond: Option<&str>,
    body: &Script,
    step: Option<&str>,
    source: &str,
    loop_depth: u32,
) {
    emit.begin_loop();
    emit.loop_test(cond);
    emit.begin_loop_body();
    walk_script(emit, body, source, loop_depth + 1);
    emit.end_loop_body();
    if let Some(step_text) = step {
        // The `next` clause is evaluated as a script each iteration; a single
        // eval-fallback is faithful and sidesteps the body's break/continue
        // scope (the step runs outside it in the structured layout).
        emit.emit_command(step_text);
    }
    emit.end_loop();
}

/// Slice `source` to a span's byte range.
fn slice(source: &str, span: Span) -> &str {
    &source[span.start() as usize..span.end() as usize]
}

/// The expression / clause source text for `span`, stripping a wrapping
/// `{…}` / `"…"` that the lexer's token span includes.
///
/// Condition and `for`-*next* token spans start at the opening brace and (per
/// the lexer) exclude the closing brace, so `source[span]` is `"{1"` for
/// `{1}` — we strip a leading `{`/`"` and an optional matching trailer to
/// recover the bare `1` an expression / script evaluator expects.
fn condition_text(source: &str, span: Span) -> &str {
    let s = slice(source, span).trim();
    if let Some(inner) = s.strip_prefix('{') {
        return inner.strip_suffix('}').unwrap_or(inner).trim();
    }
    if let Some(inner) = s.strip_prefix('"') {
        return inner.strip_suffix('"').unwrap_or(inner).trim();
    }
    s
}

/// `Some(text)` unless `text` is empty — an empty condition / step clause maps
/// to "no guard" / "no step".
fn opt(text: &str) -> Option<&str> {
    if text.is_empty() { None } else { Some(text) }
}
