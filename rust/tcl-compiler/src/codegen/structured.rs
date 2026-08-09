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

/// Depth cap for this walk's recursion over nested `if`/`while`/`for` —
/// issue #996. Unlike `loop_depth` (which tracks *loop* nesting only, for
/// `break`/`continue` validity, and never increments for `if`), this counts
/// **every** structurally-recursive level so purely `if`-nested input is
/// bounded too. Not currently reachable from any wired-up production path
/// (`structured::walk` has no caller yet — this module is WASM-backend
/// infrastructure ahead of its integration), but guarded the same way as
/// every other recursive-descent walker in this crate rather than left
/// unaudited. 256 matches the convention used elsewhere in this crate
/// (`analyser::commands::MAX_BODY_DEPTH`,
/// `lowering::MAX_LOWER_NEST_DEPTH`, `optimiser::MAX_OPTIMISER_WALK_DEPTH`)
/// since, when wired up, this runs on the native compiler's side (emitting
/// WASM, not executing inside it) — the same big-stack entry points already
/// fixed for issue #996 apply.
const MAX_STRUCTURED_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(256);

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
    walk_script(emit, script, source, 0, 0);
}

/// Emit each statement of `script` in order, stopping after one diverges (its
/// successors are dead code). `loop_depth` is the number of structurally
/// enclosing loops whose break/continue scope is open — `break`/`continue` are
/// realised structurally only when inside one (`> 0`). `depth` is this script's
/// total structural nesting level — see [`MAX_STRUCTURED_DEPTH`].
fn walk_script<E: Emit>(emit: &mut E, script: &Script, source: &str, loop_depth: u32, depth: u32) {
    for stmt in &script.statements {
        if walk_stmt(emit, stmt, source, loop_depth, depth) == Flow::Diverged {
            break;
        }
    }
}

fn walk_stmt<E: Emit>(
    emit: &mut E,
    stmt: &Statement,
    source: &str,
    loop_depth: u32,
    depth: u32,
) -> Flow {
    // Native-stack safety net — see `MAX_STRUCTURED_DEPTH`'s doc comment
    // (issue #996). Past the cap, an `if`/`while`/`for` degrades to the
    // same whole-construct eval-fallback every other unstructured
    // statement kind already uses below, instead of recursing further.
    if MAX_STRUCTURED_DEPTH.exceeded(depth)
        && matches!(
            stmt,
            Statement::If { .. } | Statement::While { .. } | Statement::For { .. }
        )
    {
        emit.emit_command(slice(source, stmt.span()));
        return Flow::Normal;
    }
    if emit.emit_typed_statement(stmt, source) {
        return if matches!(stmt, Statement::Return { .. }) {
            Flow::Diverged
        } else {
            Flow::Normal
        };
    }
    match stmt {
        Statement::If {
            clauses, else_body, ..
        } => {
            emit_if(
                emit,
                clauses,
                else_body.as_ref(),
                source,
                loop_depth,
                depth,
                stmt.span(),
            );
            Flow::Normal
        }

        Statement::While {
            condition_span,
            body,
            ..
        } => {
            let cond = condition_text(source, *condition_span);
            emit_loop(emit, opt(cond), body, None, source, loop_depth, depth);
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
            walk_script(emit, init, source, loop_depth, depth);
            let cond = condition_text(source, *condition_span);
            let step = condition_text(source, *next_span);
            emit_loop(emit, opt(cond), body, opt(step), source, loop_depth, depth);
            Flow::Normal
        }

        Statement::Return { span, .. } => {
            // Eval the `return` command so the runtime sets the result/return
            // options; its completion code (`return`, or an immediate `-level 0
            // -code`) already unwinds the function via `emit_command`'s dispatch.
            // The explicit `emit_return` makes the exit unconditional (a `return`
            // statement always leaves) and terminates this straight-line script.
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
/// `if`/`else` so the backend sees only the two-armed primitive. `depth` is
/// this level's structural nesting — see [`MAX_STRUCTURED_DEPTH`]. Each
/// `elseif` link recurses via a self-call (one native stack frame per
/// clause) independently of body nesting, so it consumes `depth` budget the
/// same way a nested body does — a pathologically long `elseif` chain is
/// exactly as dangerous as pathologically deep body nesting. `full_span` is
/// the *enclosing* `Statement::If`'s whole span (`if` through the final
/// close brace), kept only for the depth-cap fallback below.
#[allow(clippy::too_many_arguments)] // one context threaded through a recursive emit
fn emit_if<E: Emit>(
    emit: &mut E,
    clauses: &[IfClause],
    else_body: Option<&Script>,
    source: &str,
    loop_depth: u32,
    depth: u32,
    full_span: Span,
) {
    let Some((first, rest)) = clauses.split_first() else {
        return; // malformed `if` with no clauses — nothing to emit.
    };

    emit.begin_if(condition_text(source, first.condition_span));
    walk_script(emit, &first.body, source, loop_depth, depth + 1);

    if !rest.is_empty() {
        emit.begin_else();
        if MAX_STRUCTURED_DEPTH.exceeded(depth + 1) {
            // Native-stack safety net — see `MAX_STRUCTURED_DEPTH`'s doc
            // comment (issue #996). Re-running the *whole* original
            // if/elseif/else construct as one eval-fallback here (rather
            // than recursing into `emit_if` for `rest`) is semantically
            // correct: by construction this branch only runs when every
            // earlier clause's condition was false, so re-testing them
            // (cheap, and — same assumption every other eval-fallback in
            // this module already makes — side-effect-free) reaches
            // exactly the same outcome as evaluating just the remainder.
            emit.emit_command(slice(source, full_span));
        } else {
            // Remaining `elseif` clauses become a nested `if` in the `else` arm.
            emit_if(
                emit,
                rest,
                else_body,
                source,
                loop_depth,
                depth + 1,
                full_span,
            );
        }
        emit.end_if();
    } else if let Some(eb) = else_body {
        emit.begin_else();
        walk_script(emit, eb, source, loop_depth, depth + 1);
        emit.end_if();
    } else {
        emit.end_if();
    }
}

/// Drive the [`Emit`] loop protocol for a `while`/`for`. `step` is the `for`
/// *next* clause (a single eval-fallback), absent for `while`.
#[allow(clippy::too_many_arguments)] // one context threaded through a recursive emit
fn emit_loop<E: Emit>(
    emit: &mut E,
    cond: Option<&str>,
    body: &Script,
    step: Option<&str>,
    source: &str,
    loop_depth: u32,
    depth: u32,
) {
    emit.begin_loop();
    emit.loop_test(cond);
    emit.begin_loop_body();
    walk_script(emit, body, source, loop_depth + 1, depth + 1);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowering::lower_to_ir;
    use tcl_registry::CommandRegistry;

    /// A backend-free [`Emit`] that records each call as a string, so the
    /// driver's decisions can be asserted in isolation (independent of any
    /// target). This is the contract test for the "common emitter layer".
    #[derive(Default)]
    struct Recorder(Vec<String>);

    impl Emit for Recorder {
        fn emit_command(&mut self, t: &str) {
            self.0.push(format!("cmd({t})"));
        }
        fn begin_if(&mut self, c: &str) {
            self.0.push(format!("if({c})"));
        }
        fn begin_else(&mut self) {
            self.0.push("else".into());
        }
        fn end_if(&mut self) {
            self.0.push("endif".into());
        }
        fn begin_loop(&mut self) {
            self.0.push("loop{".into());
        }
        fn loop_test(&mut self, c: Option<&str>) {
            self.0.push(format!("test({})", c.unwrap_or("-")));
        }
        fn begin_loop_body(&mut self) {
            self.0.push("body{".into());
        }
        fn end_loop_body(&mut self) {
            self.0.push("}body".into());
        }
        fn end_loop(&mut self) {
            self.0.push("}loop".into());
        }
        fn emit_break(&mut self) {
            self.0.push("break".into());
        }
        fn emit_continue(&mut self) {
            self.0.push("continue".into());
        }
        fn emit_return(&mut self) {
            self.0.push("return".into());
        }
    }

    /// Lower `src` and record the structured-walk event sequence.
    fn events(src: &str) -> Vec<String> {
        let module = lower_to_ir(src, &CommandRegistry::build_default());
        let mut rec = Recorder::default();
        walk(&mut rec, &module.top_level, src);
        rec.0
    }

    /// Regression coverage for issue #996: `walk_stmt`/`emit_if`/`emit_loop`'s
    /// recursion over nested `if`/`while`/`for` (and `emit_if`'s own
    /// self-recursive `elseif`-chain walk) is now capped at
    /// `MAX_STRUCTURED_DEPTH` (256). `lower_to_ir` (called by `events`) has
    /// its own matching cap and barriers past it first in this end-to-end
    /// path, so this proves the *whole* pipeline survives deep nesting
    /// rather than isolating this module's cap specifically — same caveat
    /// as the optimiser passes' equivalent test
    /// (`optimiser::manager::tests::deeply_nested_if_survives_full_optimiser_pipeline`).
    /// Spawns its own big-stack thread since `structured::walk` has no
    /// production caller yet to inherit a big stack from — matching that
    /// test's rationale too.
    #[test]
    fn deeply_nested_if_survives_structured_walk() {
        const DEPTH: usize = 400;
        const STACK_SIZE: usize = 64 * 1024 * 1024;
        let mut src = String::new();
        for _ in 0..DEPTH {
            src.push_str("if {1} {\n");
        }
        src.push_str("set done 1\n");
        for _ in 0..DEPTH {
            src.push_str("}\n");
        }
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn(move || {
                let _ = events(&src);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// A long `elseif` chain is a *different* recursion shape from nested
    /// bodies: `emit_if` recurses once per `elseif` link via a self-call,
    /// independently of `MAX_LOWER_NEST_DEPTH` (which bounds source
    /// *nesting* depth, not chain *length* — `lowering` does not barrier
    /// this). Confirms `emit_if`'s own chain-position depth budget (issue
    /// #996) catches this shape too, not just nested bodies.
    #[test]
    fn very_long_elseif_chain_survives_structured_walk() {
        const LINKS: usize = 2000;
        const STACK_SIZE: usize = 64 * 1024 * 1024;
        let mut src = "if {0} {\n    set done 1\n}".to_owned();
        for _ in 0..LINKS {
            src.push_str(" elseif {0} {\n    set done 1\n}");
        }
        src.push_str(" else {\n    set done 1\n}\n");
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn(move || {
                let _ = events(&src);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn linear_emits_commands_in_order() {
        assert_eq!(
            events("set x 5\nputs $x\n"),
            ["cmd(set x 5)", "cmd(puts $x)"]
        );
    }

    #[test]
    fn if_else_two_arms() {
        assert_eq!(
            events("if {1} {puts a} else {puts b}\n"),
            ["if(1)", "cmd(puts a)", "else", "cmd(puts b)", "endif"],
        );
    }

    #[test]
    fn if_without_else_has_no_else_arm() {
        assert_eq!(
            events("if {1} {puts a}\n"),
            ["if(1)", "cmd(puts a)", "endif"]
        );
    }

    #[test]
    fn elseif_desugars_to_nested_if() {
        // `elseif` becomes a nested two-armed `if` in the `else` arm.
        assert_eq!(
            events("if {$a} {puts a} elseif {$b} {puts b} else {puts c}\n"),
            [
                "if($a)",
                "cmd(puts a)",
                "else",
                "if($b)",
                "cmd(puts b)",
                "else",
                "cmd(puts c)",
                "endif",
                "endif",
            ],
        );
    }

    #[test]
    fn while_drives_the_loop_protocol() {
        assert_eq!(
            events("while {$x} {puts hi}\n"),
            [
                "loop{",
                "test($x)",
                "body{",
                "cmd(puts hi)",
                "}body",
                "}loop"
            ],
        );
    }

    #[test]
    fn for_runs_init_then_loop_with_step() {
        // init runs once before the loop; the `next` clause is one eval after
        // the body scope closes (so a `continue` would run it).
        assert_eq!(
            events("for {set i 0} {$i < 3} {incr i} {puts $i}\n"),
            [
                "cmd(set i 0)",
                "loop{",
                "test($i < 3)",
                "body{",
                "cmd(puts $i)",
                "}body",
                "cmd(incr i)",
                "}loop",
            ],
        );
    }

    #[test]
    fn break_and_continue_are_completion_codes_in_a_loop() {
        assert_eq!(
            events("while {1} {if {$x} {break} else {continue}}\n"),
            [
                "loop{", "test(1)", "body{", "if($x)", "break", "else", "continue", "endif",
                "}body", "}loop",
            ],
        );
    }

    #[test]
    fn break_outside_a_loop_is_an_ordinary_command() {
        // No enclosing loop ⇒ `break` is eval-fallback'd (the runtime raises).
        assert_eq!(events("break\n"), ["cmd(break)"]);
    }

    #[test]
    fn return_diverges_and_suppresses_dead_code() {
        // The `return` command is eval'd (sets the result), then the function
        // returns; the following statement is unreachable and not emitted.
        assert_eq!(
            events("return 1\nputs after\n"),
            ["cmd(return 1)", "return"]
        );
    }

    #[test]
    fn statement_after_break_in_loop_is_dead() {
        assert_eq!(
            events("while {1} {break\nputs never}\n"),
            ["loop{", "test(1)", "body{", "break", "}body", "}loop"],
        );
    }

    #[test]
    fn foreach_is_one_opaque_command() {
        assert_eq!(
            events("foreach x {a b c} {puts $x}\n"),
            ["cmd(foreach x {a b c} {puts $x})"],
        );
    }

    #[test]
    fn empty_for_condition_means_no_guard() {
        // `for {} {} {} {body}` — empty condition ⇒ `test(-)` (unconditional).
        let ev = events("for {} {} {} {puts hi}\n");
        assert!(ev.contains(&"test(-)".to_string()), "{ev:?}");
    }
}
