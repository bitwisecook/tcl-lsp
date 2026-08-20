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
        emit.emit_command(command_text(source, stmt.span()));
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
            emit.emit_command(command_text(source, *span));
            emit.emit_return();
            Flow::Diverged
        }

        // A leaf command the backend declined; outside a loop `break` /
        // `continue` are ordinary commands too (the runtime raises "invoked …
        // outside of a loop").
        Statement::Call { span, .. } => {
            emit.emit_command(command_text(source, *span));
            Flow::Normal
        }

        // Everything else — assignments, `foreach`/`switch`/`catch`/`try`,
        // barriers, inlined blocks — is one whole-construct eval-fallback.
        other => {
            emit.emit_command(command_text(source, other.span()));
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
            emit.emit_command(command_text(source, full_span));
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

/// The **whole written command** a statement's span denotes — the text handed
/// to a whole-construct eval fallback (`tcl_eval_code`).
///
/// # Why the raw span is one byte short for a quoted final word
///
/// A statement's span is the segmenter's command span
/// ([`crate::segmenter::command_span`]): the first token's start through
/// `widen_word_end` of the last. That widening carries a deliberate **type
/// gate** — it covers a braced (`{…}`) or bracketed (`[…]`) final word, but
/// *not* a quoted one, because `cmd.range` consumers (W105 unbraced-body
/// detection, segmenter tiling) rely on the inner-end for `"…"`. So a command
/// whose last word is quoted and ends in literal text — the class whose lexer
/// span excludes the closer, per [`clause_text`]'s word-class table — loses
/// its closing `"` when the span is sliced raw:
///
/// ```text
/// "puts hi"          ->  "puts hi     (whole command is one quoted word)
/// catch "puts hi"    ->  catch "puts hi
/// return "a b"       ->  return "a b
/// ```
///
/// The runtime then raises `missing "` where real Tcl raises
/// `invalid command name "puts hi"` — a parse error on code the user wrote
/// correctly (issue #1595, the whole-command sibling of #1376).
///
/// # Deciding it from an authority, not a guess
///
/// [`clause_text`] classifies by *opener* byte because it holds a single
/// word's span. A command span covers many words and its start says nothing
/// about its **last** word, so the same trick does not transfer: the only
/// thing known here is that the closer, if one is missing, sits exactly at
/// `span.end()`.
///
/// Rather than re-deriving where the final word began — the hand-rolled scan
/// whose copies keep drifting (issues #1423, #1424) — the question is put to
/// [`tcl_lexer::script_is_complete`], the crate's `Tcl_CommandComplete` port
/// (`info complete`, verified against C Tcl 9.0.3): a truncated command is
/// *exactly* a script that needs more input, and restoring its closer is
/// exactly what makes it complete. So the widened byte is taken only when it
/// turns an incomplete script into a complete one — which also means a span
/// left short for any *other* reason (a mis-lowered dynamic body, a rebased
/// synthetic span) is never silently extended, and a command that is already
/// whole is returned byte-identical.
fn command_text(source: &str, span: Span) -> &str {
    let text = slice(source, span);
    let end = span.end() as usize;
    // `"` is the one closer `widen_word_end`'s type gate leaves out; a braced
    // or bracketed final word is already covered by the time the span is built.
    if source.as_bytes().get(end) != Some(&b'"') {
        return text;
    }
    let Some(widened) = source.get(span.start() as usize..end + 1) else {
        return text;
    };
    if tcl_lexer::script_is_complete(text) || !tcl_lexer::script_is_complete(widened) {
        return text;
    }
    widened
}

/// The expression / clause source text a condition or `for`-*next* word
/// spans.
///
/// # There is no universal span convention — classify by delimiter
///
/// Issue #1376's first fix assumed one: "the token span starts at the opener
/// and always excludes the closer". That is false, and the counter-examples
/// are ordinary code. The lexer's span geometry differs **per word class**,
/// verified against the segmenter:
///
/// | source | kind | span covers | the value |
/// |---|---|---|---|
/// | `if {$x eq {a}}` | `Str` | `{$x eq {a}` — closer excluded | `$x eq {a}` |
/// | `while {}` | `Str` | `{}` — closer **included** (empty word) | `` |
/// | `if "$x eq lit"` | `Esc` | `"$x eq lit` — closer excluded | `$x eq lit` |
/// | `if "$x"` | `Esc` | `"$x"` — closer **included** | `$x` |
/// | `while ${x}` | `Var` | `${x` — closer excluded | `${x}` |
/// | `if [foo]` | `Cmd` | `[foo` — closer excluded | `[foo]` |
/// | `while $x` | `Var` | `$x` — no delimiters | `$x` |
///
/// A quoted word's span includes its closing `"` exactly when the word *ends
/// in a substitution* (`"$x"`, `"[foo]"`, `"$a(b)"`) and excludes it when it
/// ends in literal text. Assuming the closer was always excluded emitted a
/// stray trailing `"` — `if "$a == $b"` became `$a == $b"`, which
/// `tcl_expr_bool` rejects with `missing operator` on correct user code. Only
/// `if` was affected: `lower_while` / `lower_for` gate on `arg_single` and
/// barrier a quoted condition to a whole-command eval, but `lower_if` has no
/// equivalent gate.
///
/// So: classify by the **opener byte** — a Tcl word's first byte determines
/// its class unambiguously — and take each class's bounds from the lexer owner
/// that knows that class, never from a shared assumption.
///
/// For a `${…}` / `[…]` word the delimiters belong to the **value**: `${x}`
/// and `[foo]` *are* the expression text, so the whole word is taken and
/// nothing is trimmed. Trimming them (`${x` / `[foo`) is what the first fix
/// did, because its start half skipped only `{` / `"` while its end half
/// resolved those closers too — the two halves disagreed.
///
/// `base` is the lowerer's [`crate::ir::IfClause::condition_base`], from
/// [`crate::lowering_hooks::word_content_base`]. It is consulted only for the
/// classes whose value *excludes* the opener (braced and bare), and only when
/// it lands inside the word — for every other class it is `None` by
/// construction anyway, since the segmenter reconstructs those words' text.
fn clause_text(source: &str, span: Span, base: Option<u32>) -> &str {
    let (start, span_end) = (span.start() as usize, span.end() as usize);
    let opener = source.as_bytes().get(start).copied().unwrap_or(b'\0');

    let (content_start, content_end) = match opener {
        // Substitution word: the delimiters are part of the value, so take the
        // whole word. `word_span_at` widens the span over the closer the lexer
        // left out (and returns it unchanged for a bare `$x`, which has none).
        b'$' | b'[' => (start, tcl_lexer::word_span_at(source, span).end() as usize),
        // Quoted word: locate the closing `"` with the lexer's own escape- and
        // command-substitution-aware scan rather than inferring it from the
        // span, precisely because the span may or may not include it.
        //
        // The scan runs over the whole source, so it is clamped back into the
        // span: on real input the closer is always at or before `span_end`
        // (the span either stops at it or covers it), making this a no-op, but
        // a degenerate synthetic span must not be able to slice past its own
        // word. `max(start + 1)` keeps the end from crossing the content start.
        b'"' => (
            start + 1,
            tcl_lexer::close_quote_offset(source, start)
                .unwrap_or(span_end)
                .min(span_end.max(start + 1)),
        ),
        // Braced word: the span excludes the closer except for an empty `{}`,
        // which is the one case `word_closer_offset_at` exists to get right
        // (the same trap #1423 found in `branch_folding`).
        b'{' => {
            let end = tcl_lexer::word_closer_offset_at(source, span)
                .map_or(span_end, |closer| closer as usize);
            (base_or(base, start + 1, start, end), end)
        }
        // Bare word: no delimiters, the span is the value.
        _ => (base_or(base, start, start, span_end), span_end),
    };
    source
        .get(content_start.min(content_end)..content_end)
        .unwrap_or_default()
        .trim()
}

/// The lowerer's own content anchor when it lands inside `[lo, hi]`, else
/// `fallback`. A rebased or mismatched base degrades to the class-derived
/// offset rather than slicing somewhere unrelated.
fn base_or(base: Option<u32>, fallback: usize, lo: usize, hi: usize) -> usize {
    match base.map(|b| b as usize) {
        Some(b) if b >= lo && b <= hi => b,
        _ => fallback,
    }
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

    /// Issue #1376 — [`clause_text`] unit contract, one vector per **word
    /// class**. Every span below is the lexer's real span for that source,
    /// taken from the segmenter, not from an assumed convention: the first fix
    /// for #1376 assumed a universal "the span excludes the closer" rule, and
    /// the quoted rows here are exactly the shapes that disprove it.
    #[test]
    fn clause_text_derives_content_bounds_from_the_word_class() {
        // (source, span, base, expected)
        let cases: &[(&str, u32, u32, Option<u32>, &str)] = &[
            // -- braced word: span excludes the closer --
            ("while {${x}} {b}", 6, 11, None, "${x}"),
            // …and with the lowerer's own anchor supplied.
            ("while {${x}} {b}", 6, 11, Some(7), "${x}"),
            ("if {$x eq {a}} {b}", 3, 13, None, "$x eq {a}"),
            // A *nested* empty pair at the tail is not the empty-word case —
            // the outer word's own closer still sits one byte past the span
            // (issue #1423's shape).
            ("while {$x eq {}} {b}", 6, 15, None, "$x eq {}"),
            // Surrounding whitespace inside the braces is trimmed, as before.
            ("while { $x } {b}", 6, 11, None, "$x"),
            // -- braced word, EMPTY: span *includes* the closer --
            ("while {} {b}", 6, 8, None, ""),
            // -- quoted word ending in literal text: span EXCLUDES the closer --
            (r#"if "$x eq lit" {b}"#, 3, 13, None, "$x eq lit"),
            // -- quoted word ending in a SUBSTITUTION: span INCLUDES the
            // closer. This is the shape the universal-convention assumption
            // got wrong: `if "$x"` emitted `$x"` and `tcl_expr_bool` then
            // failed with `missing operator` on correct source.
            (r#"if "$x" {b}"#, 3, 7, None, "$x"),
            (r#"if "[foo]" {b}"#, 3, 10, None, "[foo]"),
            // An escaped quote inside the word is content, not the closer —
            // the scan is escape-aware, so neither end is misplaced.
            (r#"if "$x eq \"a\"" {b}"#, 3, 15, None, r#"$x eq \"a\""#),
            // The quoted arm locates the closer itself and does **not** read
            // the span's end, which is the whole reason it survives a span
            // that includes the closer and one that does not. Same source,
            // both span ends, same answer — a regression to span-derived
            // trimming breaks one of these two rows.
            (r#"if "$x" {b}"#, 3, 7, None, "$x"), // end past the closer
            (r#"if "$x" {b}"#, 3, 6, None, "$x"), // end before the closer
            // -- substitution word: the delimiters ARE part of the value --
            ("while ${x} {b}", 6, 9, None, "${x}"),
            ("if [foo] {b}", 3, 7, None, "[foo]"),
            // A `}` inside the name does not end the word early under 9.x.
            ("if ${a{b}c} {b}", 3, 10, None, "${a{b}c}"),
            // -- bare word: no delimiters, the span is the value --
            ("while $c {b}", 6, 8, None, "$c"),
            ("while 1 {b}", 6, 7, None, "1"),
            // A bare word that merely *starts* with `$` is still whole.
            ("while $x+1 {b}", 6, 10, None, "$x+1"),
            // -- base handling --
            // An out-of-range base is ignored in favour of the class-derived
            // start rather than slicing somewhere unrelated.
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

    /// Issue #1595 — [`command_text`] unit contract. The rows split into the
    /// truncated class (a final quoted word ending in literal text, which the
    /// segmenter's type gate leaves one byte short) and the majority that must
    /// come back byte-identical.
    #[test]
    fn command_text_restores_only_a_missing_final_quote() {
        // (source, span, expected)
        let cases: &[(&str, u32, u32, &str)] = &[
            // -- the truncated class: the closer sits at `span.end()` --
            (r#""puts hi""#, 0, 8, r#""puts hi""#),
            (r#"catch "puts hi""#, 0, 14, r#"catch "puts hi""#),
            (r#"return "a b""#, 0, 11, r#"return "a b""#),
            // -- quoted final word ending in a substitution: the span already
            // covers the closer, so nothing is added --
            (r#"catch "puts $x""#, 0, 15, r#"catch "puts $x""#),
            // -- braced / bracketed final words: already widened upstream --
            (
                "foreach x {a b} {puts $x}",
                0,
                25,
                "foreach x {a b} {puts $x}",
            ),
            ("catch [foo]", 0, 11, "catch [foo]"),
            // …and the type gate is a real gate, not decoration: a `}` or `]`
            // sitting at the span end is *not* restored here, because a span
            // that reaches this helper has already been widened over those by
            // `widen_word_end`. Completing the script is necessary but not
            // sufficient — the closer has to be the one the gate omits.
            ("catch {a}", 0, 8, "catch {a"),
            ("catch [foo]", 0, 10, "catch [foo"),
            // -- a `"` sitting at the span end that closes nothing: the
            // completeness pair refuses the widening rather than guessing.
            // Mid-word (`a"b`) the quote is literal, so the text is already a
            // complete script and stays as it is.
            (r#"puts a"b""#, 0, 8, r#"puts a"b"#),
            // A span left short for some *other* reason is not extended just
            // because a `"` sits at its end — adding it still leaves the brace
            // open, so the widened form is not complete either.
            (r#"puts {a"b}"#, 0, 7, r"puts {a"),
            // A degenerate span never slices past the buffer.
            (r#"catch "x""#, 0, 99, ""),
        ];
        for &(source, start, end, expected) in cases {
            let span = Span::new(start, end);
            assert_eq!(
                command_text(source, span),
                expected,
                "command_text({source:?}, {start}..{end})"
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
