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
/// bounded too. This walk **is** on a wired-up production path — the WASM
/// backend drives it for the top level and for every proc body
/// (`codegen::wasm::backend::codegen`, `backend.rs:1919` and `:1961`) — and it
/// is guarded the same way as every other recursive-descent walker in this
/// crate. (A stale claim that `structured::walk` "has no caller yet" stood
/// here until issue #1376; it is plausibly why neither `slice` nor the clause
/// text helper was hardened.) 256 matches the convention used elsewhere in
/// this crate (`analyser::commands::MAX_BODY_DEPTH`,
/// `lowering::MAX_LOWER_NEST_DEPTH`, `optimiser::MAX_OPTIMISER_WALK_DEPTH`):
/// this runs on the native compiler's side (emitting WASM, not executing
/// inside it) — the same big-stack entry points already fixed for issue #996
/// apply.
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
    // `break` / `continue` inside a loop are completion codes the walk realises
    // as structured jumps. That decision belongs to the walk, not to a backend's
    // statement emission, so it is taken before the typed-statement seam — a
    // backend that can compile the call would otherwise route the jump through
    // its own completion dispatch instead.
    if loop_depth > 0
        && let Statement::Call { command, .. } = stmt
    {
        if command == "break" {
            emit.emit_break();
            return Flow::Diverged;
        }
        if command == "continue" {
            emit.emit_continue();
            return Flow::Diverged;
        }
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
            condition_base,
            body,
            ..
        } => {
            let cond = clause_text(source, *condition_span, *condition_base);
            emit_loop(emit, opt(cond), body, None, source, loop_depth, depth);
            Flow::Normal
        }

        Statement::For {
            init,
            condition_span,
            condition_base,
            next_span,
            body,
            raw_args,
            ..
        } => {
            // The init clause runs once, in the enclosing scope.
            walk_script(emit, init, source, loop_depth, depth);
            let cond = clause_text(source, *condition_span, *condition_base);
            // Issue #1376 residual: `Statement::For` carries `condition_base`
            // but no `next_base`, so the step clause takes the lowerer's own
            // de-braced word text (`raw_args[2]` — `for init cond next body`)
            // when it is present, which is the IR fact that corresponds to
            // `condition_base` for this word. Synthetically built loops (GVN,
            // static-loop unrolling, inlining) carry no `raw_args`, so those
            // fall back to the span-derived content.
            let step = match raw_args.get(2) {
                Some(text) => text.trim(),
                None => clause_text(source, *next_span, None),
            };
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

        // A leaf command the backend declined; outside a loop `break` /
        // `continue` are ordinary commands too (the runtime raises "invoked …
        // outside of a loop").
        Statement::Call { span, .. } => {
            emit.emit_command(slice(source, *span));
            Flow::Normal
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

    emit.begin_if(clause_text(
        source,
        first.condition_span,
        first.condition_base,
    ));
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

/// Slice `source` to a span's byte range. A span past the end of `source`
/// or landing off a UTF-8 character boundary degrades to an empty slice
/// rather than panicking — a bad IR span (e.g. from a mis-lowered
/// dynamic body, issue #1375) must not abort the compiler.
fn slice(source: &str, span: Span) -> &str {
    source
        .get(span.start() as usize..span.end() as usize)
        .unwrap_or_default()
}

/// The expression / clause source text a condition or `for`-*next* word
/// spans, driven by the anchors the CST already produced.
///
/// Issue #1376: a clause word's token span starts at the **opening**
/// delimiter and ends *exclusively at the closing* one, so `source[span]` is
/// `"{1"` for `{1}`. The content is therefore `source[content_start..span.end()]`
/// with nothing to strip off the tail — an earlier `strip_suffix('}')` here
/// could only ever delete a byte of real content, truncating every clause
/// whose last inner character is `}` (`${name}`, a trailing braced word, a
/// nested dict/list literal) or `"`.
///
/// `base` is the lowerer's [`crate::ir::IfClause::condition_base`] — the
/// absolute offset of the content's first byte, computed once by
/// [`crate::lowering_hooks::word_content_base`], which is the one place the
/// opening-delimiter width is recovered without guessing. When it is absent
/// (a reconstructed / multi-token word, an empty `{}`, or a synthetically
/// built statement) the opener width is recovered from the opener byte the
/// span points at — directly observable, not inferred, since a Tcl word that
/// begins with `{` or `"` is a braced / quoted word and no other word may.
///
/// The *end* is never guessed at all: it comes from
/// [`tcl_lexer::word_closer_offset_at`], the shared owner of the inner-end
/// convention. That matters because the convention has one exception — an
/// empty `{}` / `""` span already covers its closer — which a plain
/// `span.end()` would render as the literal text `"}"` (issue #1423 is the
/// same trap in `branch_folding`). A bare or unterminated word has no closer,
/// and the span's exclusive end is already the content end.
fn clause_text(source: &str, span: Span, base: Option<u32>) -> &str {
    let start = span.start();
    // For a delimited word this is the closer's offset, i.e. the exclusive
    // end of the content — which is *not* `span.end()` for an empty `{}`.
    let end = tcl_lexer::word_closer_offset_at(source, span).unwrap_or_else(|| span.end());
    let content_start = match base {
        // Trust the lowerer's anchor only when it lands inside this word; a
        // rebased or mismatched base degrades to the span-derived form rather
        // than slicing somewhere unrelated.
        Some(b) if b >= start && b <= end => b,
        _ => {
            let opener = source
                .as_bytes()
                .get(start as usize)
                .copied()
                .unwrap_or(b'\0');
            if matches!(opener, b'{' | b'"') {
                start.saturating_add(1).min(end)
            } else {
                start
            }
        }
    };
    source
        .get(content_start as usize..end as usize)
        .unwrap_or_default()
        .trim()
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

    /// Issue #1376 — [`clause_text`] unit contract. The token span starts at
    /// the opener and ends *exclusively at* the closer, so nothing may ever be
    /// stripped off the tail. Each vector below is a byte range constructed
    /// the way the lexer builds one, so a reintroduced `strip_suffix` fails
    /// here even if no lowering path currently produces the shape.
    #[test]
    fn clause_text_never_strips_a_closer_the_span_already_excludes() {
        // (source, span, base, expected)
        let cases: &[(&str, u32, u32, Option<u32>, &str)] = &[
            // Braced, trailing `}` is real content — the #1376 repro.
            ("while {${x}} {b}", 6, 11, None, "${x}"),
            // …and with the lowerer's own anchor supplied.
            ("while {${x}} {b}", 6, 11, Some(7), "${x}"),
            // Braced, nested braced word at the tail.
            ("if {$x eq {a}} {b}", 3, 13, None, "$x eq {a}"),
            // Quoted: the opener is `"`, the closer is likewise excluded, and
            // a trailing `\"` in the content must survive.
            (r#"while "$x eq \"a\"" {b}"#, 6, 18, None, r#"$x eq \"a\""#),
            // Bare word: no delimiter at all, content is the whole span.
            ("while $c {b}", 6, 8, None, "$c"),
            // Empty braced clause — the one exception to the inner-end span
            // convention: the span already covers its closer, so a naive
            // `span.end()` would yield the literal text `}`.
            ("while {} {b}", 6, 8, None, ""),
            // …likewise an empty quoted clause.
            (r#"while "" {b}"#, 6, 8, None, ""),
            // A *nested* empty pair at the tail is not the empty-word case —
            // the outer word's own closer still sits one byte past the span
            // (issue #1423's shape).
            ("while {$x eq {}} {b}", 6, 15, None, "$x eq {}"),
            // Surrounding whitespace inside the braces is trimmed, as before.
            ("while { $x } {b}", 6, 11, None, "$x"),
            // An out-of-range base is ignored in favour of the span-derived
            // opener rather than slicing somewhere unrelated.
            ("while {${x}} {b}", 6, 11, Some(99), "${x}"),
        ];
        for &(source, start, end, base, expected) in cases {
            let span = Span::new(start, end);
            assert_eq!(
                clause_text(source, span, base),
                expected,
                "clause_text({source:?}, {start}..{end}, {base:?})"
            );
        }
    }

    /// The `for`-*next* clause has no `next_base`, so `walk_stmt` drives it
    /// from the lowerer's `raw_args[2]`. Proven end-to-end through the real
    /// lowering so the argument index is checked against the IR, not assumed.
    #[test]
    fn for_step_clause_is_emitted_whole() {
        let ev = events("for {set i 0} {$i<2} {set x ${i}} {puts a}\n");
        assert!(
            ev.iter().any(|e| e == "cmd(set x ${i})"),
            "the `for` step must be emitted whole: {ev:?}"
        );
        assert!(
            ev.iter().any(|e| e == "test($i<2)"),
            "the `for` condition must be emitted whole: {ev:?}"
        );
    }

    /// The `while` and `if` clause sites, through the real lowering.
    #[test]
    fn while_and_if_conditions_are_emitted_whole() {
        let ev = events("while {${x}} {puts hi}\n");
        assert!(
            ev.iter().any(|e| e == "test(${x})"),
            "the `while` condition must be emitted whole: {ev:?}"
        );
        let ev = events("if {$x eq {a}} {puts a}\n");
        assert!(
            ev.iter().any(|e| e == "if($x eq {a})"),
            "the `if` condition must be emitted whole: {ev:?}"
        );
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
