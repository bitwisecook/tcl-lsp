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

//! Structured IR lowering for control-flow commands.
//!
//! Each method converts a segmented command into its corresponding
//! structured IR statement (`If`, `For`, `While`, `Foreach`, `Catch`,
//! `Try`, `Switch`, `dict` subcommands).

use std::borrow::Cow;

use tcl_lexer::{Lexer, SourceMap, Span, Token, TokenType};

use crate::expr_parser::parse_expr;
use crate::ir::{ForeachIterator, IfClause, Script, Statement, SwitchArm, SwitchMode, TryHandler};
use crate::lowering_hooks::word_content_base;
use crate::naming::normalise_var_name;
use crate::segmenter::SegmentedCommand;

use crate::segmenter::word_piece;

use super::{Lowerer, parse_param_names};

/// Parse a braced switch body into a flat list of word elements.
///
/// Tokenises the body text and collects words separated by whitespace/EOL,
/// merging multi-token words. Returns the element texts in order.
/// A single pattern/body pair collected by
/// [`lower_switch`](super::Lowerer::lower_switch) before the
/// final `SwitchArm` list is built. Factored out to keep the
/// pair vector's type signature readable.
struct SwitchPair {
    pattern: String,
    pattern_span: Span,
    body_text: String,
    body_span: Option<Span>,
    body_arg_idx: Option<usize>,
}

/// Split a switch's braced body into its `(element_text,
/// local_span)` pairs. The span is expressed in the body text's
/// own offset space — callers relocate it to the full source
/// buffer by adding the body text's starting offset.
fn switch_body_elements(body_text: &str) -> Vec<(String, Span)> {
    let sm = SourceMap::new(body_text);
    let lexer = Lexer::new(body_text);
    let Ok(tokens) = lexer.tokenise_all() else {
        return Vec::new();
    };

    let mut elements: Vec<(String, Span)> = Vec::new();
    let mut prev_is_sep = true;

    for &tok in &tokens {
        match tok.kind {
            TokenType::Sep | TokenType::Eol | TokenType::Comment => {
                prev_is_sep = true;
                continue;
            }
            TokenType::Eof => continue,
            _ => {}
        }

        let piece = word_piece(&sm, tok);
        // Token spans for braced / quoted words do not include
        // the trailing `}` / `"` — the lexer treats the closing
        // delimiter as an anchor rather than a span member.
        // Extend `end` by one byte when the source byte at the
        // current end position is the matching closer, so the
        // resulting span covers the whole `{…}` / `"…"` word.
        let tok_span = adjusted_delim_span(body_text, tok);
        if prev_is_sep {
            elements.push((piece, tok_span));
        } else if let Some(last) = elements.last_mut() {
            last.0.push_str(&piece);
            last.1 = Span::new(last.1.start(), tok_span.end());
        } else {
            elements.push((piece, tok_span));
        }
        prev_is_sep = false;
    }

    elements
}

/// Extend a token's span by one byte when its end lands on a
/// closing `{}` / `""` delimiter — the lexer stops the span
/// just before the closer. For all other tokens the span is
/// returned unchanged.
fn adjusted_delim_span(source: &str, tok: tcl_lexer::Token) -> Span {
    if tok.content_offset == 0 {
        return tok.span;
    }
    let end = tok.span.end() as usize;
    let bytes = source.as_bytes();
    if end < bytes.len() && matches!(bytes[end], b'}' | b'"') {
        Span::new(tok.span.start(), tok.span.end() + 1)
    } else {
        tok.span
    }
}

/// Parse switch options, returning `(first_non_option_index, mode, nocase,
/// unknown)`. `unknown` is set when a leading `-word` is not one of the options
/// the compiler inlines (`-exact`/`-glob`/`-regexp`/`-nocase`/`--`) — an
/// arg-taking `-indexvar`/`-matchvar`, or an invalid option such as `-foo`. The
/// caller bails the whole switch to the runtime command, which validates the
/// option set (tclsh rejects `-foo`) and handles the side-channel writes.
fn parse_switch_options(args: &[String]) -> (usize, SwitchMode, bool, bool) {
    let mut i = 0;
    let mut mode = SwitchMode::Exact;
    let mut nocase = false;
    let mut unknown = false;
    while i < args.len() && args[i].starts_with('-') {
        match args[i].as_str() {
            "--" => {
                i += 1;
                break;
            }
            "-exact" => mode = SwitchMode::Exact,
            "-glob" => mode = SwitchMode::Glob,
            "-regexp" => mode = SwitchMode::Regexp,
            "-nocase" => nocase = true,
            _ => {
                unknown = true;
                break;
            }
        }
        i += 1;
    }
    (i, mode, nocase, unknown)
}

/// Whether a loop body (or `for` next-clause) redefines `break`/`continue` via
/// `proc break …` / `proc continue …`. Such a loop is compiled through the
/// runtime builtin (a barrier) rather than the inline JUMP fast-path: the
/// builtin dispatches break/continue as commands, honouring the redefinition,
/// whereas the inline JUMP fires unconditionally and would loop forever
/// (proc-7.3, Bug 729692). A conservative substring test — a false positive
/// merely takes the correct (if slower) runtime path.
fn redefines_loop_control(body: &str) -> bool {
    body.contains("proc break") || body.contains("proc continue")
}

/// The expression text a condition word is parsed from: `text` as-is, except
/// a lone *bare* `$name` Var token, whose `word_piece` reconstruction
/// re-braces to `${name}` for multi-piece concatenation safety.  A
/// single-token condition word has no following piece to glue onto, and the
/// re-braced spelling leaks into rendered diagnostics (I230 quoting `${n}`
/// where the source spells `$n`) — restore the source's bare form.  Also
/// re-aligns the text with the token's span so `word_content_base` can
/// anchor it.
fn condition_source_text<'t>(tok: Option<&Token>, single: bool, text: &'t str) -> Cow<'t, str> {
    let Some(tok) = tok else {
        return Cow::Borrowed(text);
    };
    if single
        && tok.kind == TokenType::Var
        && tok.content_offset == 1
        && let Some(inner) = text.strip_prefix("${").and_then(|t| t.strip_suffix('}'))
    {
        return Cow::Owned(format!("${inner}"));
    }
    Cow::Borrowed(text)
}

impl Lowerer<'_> {
    // if

    /// Lower `if cond body ?elseif cond body ...? ?else body?`.
    pub(super) fn lower_if(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();

        if args.is_empty() {
            return Self::barrier(seg, "malformed if");
        }

        let mut clauses = Vec::new();
        let mut else_body = None;
        let mut else_span = None;
        let mut i = 0;
        // Reachability tracking. Once a clause's condition
        // folds to a static `true`, every later clause + the
        // ``else`` branch is dead. A clause whose own condition
        // is a static `false` is dead this iteration but not
        // necessarily later — track per-clause via the
        // ``dead_code_depth`` counter.
        let mut later_clauses_dead = false;

        while i < args.len() {
            if args[i] == "elseif" {
                i += 1;
                // `elseif` must be followed by a condition; a dangling `elseif`
                // (`if 1 {a} elseif`) is "no expression after elseif". Defer to the
                // runtime `if`, which reports it faithfully (if-2.3).
                if i >= args.len() {
                    return Self::barrier(seg, "if missing elseif expression");
                }
                continue;
            }
            if args[i] == "else" {
                if i + 1 >= args.len() {
                    return Self::barrier(seg, "malformed if else clause");
                }
                // Exactly one body may follow `else`; trailing words
                // (`if 0 {a} else {b} junk`) are "extra words after else" — defer to
                // the runtime `if` (if-3.5).
                if i + 2 < args.len() {
                    return Self::barrier(seg, "if extra words after else");
                }
                // Only a substitution-free literal body inlines (see the
                // clause-body note below).
                if !super::seg_word_is_static_literal(seg, i + 2) {
                    return Self::barrier(seg, "if with non-literal body");
                }
                let body_tok = arg_tokens.get(i + 1);
                let dead = later_clauses_dead;
                if dead {
                    self.dead_code_depth += 1;
                }
                else_body = Some(self.lower_body_from_tok(&args[i + 1], body_tok, namespace));
                if dead {
                    self.dead_code_depth -= 1;
                }
                else_span = body_tok.map(|t| t.span);
                break;
            }

            let cond_idx = i;
            i += 1;
            if i < args.len() && args[i] == "then" {
                i += 1;
            }
            if i >= args.len() {
                return Self::barrier(seg, "malformed if clause");
            }

            let body_idx = i;
            // C's TclCompileIfCmd only inlines a braced-literal body; a body
            // carrying substitutions (`$x`, `[cmd]`, a quoted or concatenated
            // word like `$x1$x2`) must be substituted *then* evaluated as a
            // script — which the runtime `if` command does. Bail the whole
            // construct to that command rather than mis-parsing the unsubstituted
            // word as a literal script at compile time.
            if !super::seg_word_is_static_literal(seg, body_idx + 1) {
                return Self::barrier(seg, "if with non-literal body");
            }
            let body_tok = arg_tokens.get(body_idx);
            let cond_tok = arg_tokens.get(cond_idx);
            let static_cond = super::static_bool(&args[cond_idx]);
            let clause_dead = later_clauses_dead || matches!(static_cond, Some(false));
            if clause_dead {
                self.dead_code_depth += 1;
            }
            let body = self.lower_body_from_tok(&args[body_idx], body_tok, namespace);
            if clause_dead {
                self.dead_code_depth -= 1;
            }
            let cond_single = arg_single.get(cond_idx).copied().unwrap_or(false);
            let cond_text = condition_source_text(cond_tok, cond_single, &args[cond_idx]);
            clauses.push(IfClause {
                condition: parse_expr(&cond_text, self.dialect),
                condition_span: cond_tok.map_or(seg.span, |t| t.span),
                condition_base: cond_tok
                    .and_then(|t| word_content_base(t.span, cond_single, &cond_text)),
                body,
                body_span: body_tok.map_or(seg.span, |t| t.span),
            });
            // Static-true condition latches the dead-code flag so
            // remaining clauses + the else branch are suppressed.
            if matches!(static_cond, Some(true)) {
                later_clauses_dead = true;
            }
            i += 1;
            // After a clause, only `elseif` / `else` (or end) may follow. A bare
            // word (`if 1<2 {a} elwood {b}`, `if 0 {a} {b}`) is "extra words
            // after else clause" — the inline loop would otherwise mis-read it
            // as another implicit clause. Bail to the runtime `if`, which
            // reports the error faithfully.
            if i < args.len() && args[i] != "elseif" && args[i] != "else" {
                return Self::barrier(seg, "if with extra words");
            }
        }

        if clauses.is_empty() {
            return Self::barrier(seg, "malformed if");
        }

        Statement::If {
            span: seg.span,
            clauses,
            else_body,
            else_span,
        }
    }

    // for

    /// Lower `for init cond next body`.
    pub(super) fn lower_for(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();

        // `for` takes exactly four arguments; a wrong count barriers to the
        // runtime builtin, which raises `wrong # args` (for-old-1.7). Lowering a
        // ≥4 form would silently drop the extras and run arg[0] as the body.
        if args.len() != 4 || arg_tokens.len() < 4 {
            return Self::barrier(seg, "malformed for");
        }
        if !(arg_single[0] && arg_single[1] && arg_single[2] && arg_single[3]) {
            return Self::barrier(seg, "for with dynamic arguments");
        }
        // A body/next that redefines break/continue must run through the runtime
        // builtin, which dispatches them (so the redefinition is honoured) rather
        // than firing the compiled JUMP fast-path unconditionally and looping
        // forever (proc-7.3).
        if redefines_loop_control(&args[2]) || redefines_loop_control(&args[3]) {
            return Self::barrier(seg, "for redefines break/continue");
        }

        let init = self.lower_body_from_tok(&args[0], Some(&arg_tokens[0]), namespace);
        let next = self.lower_body_from_tok(&args[2], Some(&arg_tokens[2]), namespace);
        let body = self.lower_body_from_tok(&args[3], Some(&arg_tokens[3]), namespace);

        Statement::For {
            span: seg.span,
            init,
            init_span: arg_tokens[0].span,
            condition: parse_expr(
                &condition_source_text(arg_tokens.get(1), arg_single[1], &args[1]),
                self.dialect,
            ),
            condition_span: arg_tokens[1].span,
            condition_base: word_content_base(
                arg_tokens[1].span,
                arg_single[1],
                &condition_source_text(arg_tokens.get(1), arg_single[1], &args[1]),
            ),
            next,
            next_span: arg_tokens[2].span,
            body,
            body_span: arg_tokens[3].span,
            raw_args: args.to_vec(),
            raw_tokens: Some(Self::cmd_tokens(seg)),
        }
    }

    // while

    /// Lower `while cond body`.
    pub(super) fn lower_while(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();

        // `while` takes exactly two arguments; a wrong count barriers to the
        // runtime builtin, which raises `wrong # args` (while-old-4.3).
        if args.len() != 2 || arg_tokens.len() < 2 {
            return Self::barrier(seg, "malformed while");
        }
        if !(arg_single[0] && arg_single[1]) {
            return Self::barrier(seg, "while with dynamic arguments");
        }
        // See `lower_for`: a body redefining break/continue must use the runtime
        // builtin so the redefinition is dispatched (proc-7.3).
        if redefines_loop_control(&args[1]) {
            return Self::barrier(seg, "while redefines break/continue");
        }

        let body = self.lower_body_from_tok(&args[1], Some(&arg_tokens[1]), namespace);

        Statement::While {
            span: seg.span,
            condition: parse_expr(
                &condition_source_text(arg_tokens.first(), arg_single[0], &args[0]),
                self.dialect,
            ),
            condition_span: arg_tokens[0].span,
            condition_base: word_content_base(
                arg_tokens[0].span,
                arg_single[0],
                &condition_source_text(arg_tokens.first(), arg_single[0], &args[0]),
            ),
            body,
            body_span: arg_tokens[1].span,
            raw_args: args.to_vec(),
            raw_tokens: Some(Self::cmd_tokens(seg)),
        }
    }

    // foreach / lmap

    /// Lower `foreach varList list ?varList list ...? body`.
    pub(super) fn lower_foreach(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        is_lmap: bool,
    ) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();

        if args.len() < 3 || args.len().is_multiple_of(2) {
            return Self::barrier(seg, "malformed foreach");
        }

        let body_idx = args.len() - 1;
        let body_tok = arg_tokens.get(body_idx);
        if body_tok.is_none() || body_idx >= arg_single.len() || !arg_single[body_idx] {
            return Self::barrier(seg, "foreach with dynamic body");
        }

        let mut iterators = Vec::new();
        for i in (0..body_idx).step_by(2) {
            let var_names = parse_param_names(&args[i]);
            iterators.push(ForeachIterator {
                vars: var_names,
                list_arg: args[i + 1].clone(),
            });
        }

        let body = self.lower_body_from_tok(&args[body_idx], body_tok, namespace);

        // Route a loop to its runtime builtin on the bytecode path when the inline
        // codegen can't compile it correctly:
        //   * `lmap` with a *branching* body — the inline collector
        //     (`LMAP_COLLECT` on the body's single fall-through tail) can only
        //     gather from a straight-line body; a body with an `if`/`while`/
        //     `switch`/nested loop or an unwinding `return` compiles to a
        //     multi-block CFG it can't collect from, so it stays on the runtime
        //     `lmap` (correct, though `yield` can't cross it). A straight-line
        //     `lmap` lowers inline — yieldable and correctly collecting.
        //   * a `foreach` whose body *directly* contains another `foreach`/`lmap` —
        //     the inner loop's back-edge corrupts the outer's `FOREACH_STEP`
        //     routing (the nested-complex-foreach bug). A loop nested via an
        //     `if`/`while`/`for` is unaffected and stays inline.
        // The runtime builtin evaluates the body transparently (an inner loop
        // recompiles fresh), so nesting works by recursion.
        let body_nests_foreach = body
            .statements
            .iter()
            .any(|s| matches!(s, Statement::Foreach { .. }));
        let lmap_needs_runtime = is_lmap && !Self::body_is_straight_line(&body);
        if self.target.is_bytecode() && (lmap_needs_runtime || body_nests_foreach) {
            return Self::barrier(seg, if is_lmap { "lmap" } else { "foreach" });
        }

        Statement::Foreach {
            span: seg.span,
            iterators,
            body,
            body_span: body_tok.map_or(seg.span, |t| t.span),
            is_lmap,
            raw_args: args.to_vec(),
            is_dict_iteration: false,
            is_array_iteration: false,
            raw_tokens: Some(Self::cmd_tokens(seg)),
        }
    }

    /// A loop body is *straight-line* when every statement compiles to a single
    /// fall-through block — no branch, join, or unwinding `return`. That is the
    /// shape the inline collecting-`lmap` codegen needs: it strips the body's
    /// trailing `POP` and appends the result via one `LMAP_COLLECT` on the
    /// fall-through tail, so a branch/join (an `if`/`while`/`switch`/nested loop)
    /// or a `return` that unwinds past the collect point would drop or mis-gather
    /// results. Such a body keeps `lmap` on the runtime builtin.
    ///
    /// A bare `break`/`continue` is also excluded: in a *simple* foreach body it
    /// jumps to the loop header (which re-runs `FOREACH_START`) rather than
    /// `FOREACH_STEP` — a pre-existing simple-foreach limitation that predates and
    /// is orthogonal to collection — so a collecting body carrying one stays on
    /// the runtime `lmap`, which handles loop control correctly.
    fn body_is_straight_line(body: &Script) -> bool {
        body.statements.iter().all(|s| match s {
            Statement::Call { command, .. } => command != "break" && command != "continue",
            Statement::AssignConst { .. }
            | Statement::AssignExpr { .. }
            | Statement::AssignValue { .. }
            | Statement::Incr { .. }
            | Statement::ExprEval { .. }
            | Statement::Barrier { .. } => true,
            _ => false,
        })
    }

    /// Lower `foreachLine varName filename body` (Tcl 9.0+, TIP 670)
    /// as a single-iterator [`Statement::Foreach`] so variables
    /// assigned inside the body propagate to the enclosing scope —
    /// matching plain `foreach`'s lattice behaviour rather than the
    /// opaque [`Statement::Barrier`] treatment used for generic
    /// stdlib procs.
    ///
    /// # Analyser IR only
    ///
    /// The resulting [`Statement::Foreach`] is for **static-analysis
    /// dataflow only** — it gives the analyser the same lattice
    /// shape it would see for a real `foreach` loop (body-scope
    /// variable propagation, def-use over the iteration variable,
    /// W-code emitters that walk loop bodies, etc.).  The runtime
    /// semantics of `foreachLine` are different: it reads lines from
    /// a file rather than iterating a Tcl list.
    ///
    /// **Downstream codegen / runtime-emission consumers MUST NOT
    /// treat this IR as a real list-iteration `foreach`.**  A codegen
    /// consumer must detect `raw_args[0] == "foreachLine"` (or
    /// equivalent) before treating this as a list iteration.
    pub(super) fn lower_foreach_line(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
    ) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();

        // `foreachLine varName filename body` — exactly three args.
        if args.len() != 3 {
            return Self::barrier(seg, "malformed foreachLine");
        }

        // Body must be a single static brace-string literal; dynamic
        // bodies (`$body`, `[cmd]`, multi-token) fall through to the
        // runtime command via `Statement::Barrier`.  Mirrors the
        // `lower_catch` body guard — a `Var` / `Cmd` single-token
        // word is still dynamic and must not be compiled as a
        // static loop body.
        let body_tok = arg_tokens.get(2);
        let body_is_braced_literal = body_tok.is_some_and(|t| t.kind == TokenType::Str);
        if !body_is_braced_literal || arg_single.get(2).copied() != Some(true) {
            return Self::barrier(seg, "foreachLine with dynamic body");
        }

        // Single iterator binding the loop variable.  `list_arg`
        // semantically carries "the iteration source" — for plain
        // `foreach` that's the list; for `foreachLine` it's the
        // filename (the runtime reads lines from it).  Downstream
        // dataflow doesn't care: the lattice-propagation matters,
        // not the literal value.  See the type-level doc-comment
        // above for the runtime-semantics caveat.
        let iterators = vec![ForeachIterator {
            vars: parse_param_names(&args[0]),
            list_arg: args[1].clone(),
        }];

        let body = self.lower_body_from_tok(&args[2], body_tok, namespace);

        Statement::Foreach {
            span: seg.span,
            iterators,
            body,
            body_span: body_tok.map_or(seg.span, |t| t.span),
            is_lmap: false,
            raw_args: args.to_vec(),
            is_dict_iteration: false,
            is_array_iteration: false,
            raw_tokens: Some(Self::cmd_tokens(seg)),
        }
    }

    // catch

    /// Lower `catch body ?resultVar? ?optionsVar?`.
    pub(super) fn lower_catch(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();

        if args.is_empty() {
            return Self::barrier(seg, "malformed catch");
        }
        // Body must be a single brace-literal (`Str` kind) token to
        // compile statically.  Variable references (`$cmd`) and
        // bracket commands (`[expr ...]`) are single-token but
        // non-`Str` and must fall through to the runtime
        // `eval_catch`, which calls `eval_script` on the substituted
        // value.  Without the kind check, ``catch $cmd res`` would
        // be compiled as "call the proc named by ``$cmd``" — wrong.
        if arg_tokens.is_empty()
            || !arg_single.first().copied().unwrap_or(false)
            || arg_tokens[0].kind != TokenType::Str
        {
            return Self::barrier(seg, "catch with dynamic body");
        }

        let body = self.lower_body_from_tok(&args[0], Some(&arg_tokens[0]), namespace);
        let result_var = args.get(1).map(|a| normalise_var_name(a).to_owned());
        let options_var = args.get(2).map(|a| normalise_var_name(a).to_owned());

        Statement::Catch {
            span: seg.span,
            body,
            body_span: arg_tokens[0].span,
            result_var,
            options_var,
            raw_args: args.to_vec(),
            tokens: Some(Self::cmd_tokens(seg)),
        }
    }

    // try

    /// Lower `try body ?on|trap matchArg varList handlerBody ...? ?finally finallyBody?`.
    pub(super) fn lower_try(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        // The bytecode/VM compile path lowers `try` to a runtime-command barrier:
        // the backend has no exception-range support, so a structured `try` can't
        // be compiled correctly (its handler/finally clauses would be dropped).
        // Analysis callers keep the structured form below. See `CompileTarget`.
        if self.target.is_bytecode() {
            return Self::barrier(seg, "try");
        }

        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();

        if args.is_empty() {
            return Self::barrier(seg, "malformed try");
        }
        if arg_tokens.is_empty() || !arg_single.first().copied().unwrap_or(false) {
            return Self::barrier(seg, "try with dynamic body");
        }

        let body = self.lower_body_from_tok(&args[0], Some(&arg_tokens[0]), namespace);
        let mut handlers = Vec::new();
        let mut finally_body = None;
        let mut finally_span = None;

        let mut i = 1;
        while i < args.len() {
            let keyword = &args[i];

            if keyword == "finally" && i + 1 < args.len() {
                let fin_tok = arg_tokens.get(i + 1);
                let fin_single = arg_single.get(i + 1).copied().unwrap_or(false);
                if fin_tok.is_some() && fin_single {
                    finally_body = Some(self.lower_body_from_tok(&args[i + 1], fin_tok, namespace));
                    finally_span = fin_tok.map(|t| t.span);
                }
                i += 2;
                continue;
            }

            if (keyword == "on" || keyword == "trap") && i + 3 < args.len() {
                let match_arg = args[i + 1].clone();
                let var_list = &args[i + 2];
                let handler_tok = arg_tokens.get(i + 3);
                let handler_single = arg_single.get(i + 3).copied().unwrap_or(false);

                let var_names = parse_param_names(var_list);
                let result_var = var_names.first().map(|v| normalise_var_name(v).to_owned());
                let options_var = var_names.get(1).map(|v| normalise_var_name(v).to_owned());

                // A handler body of literal `-` is a fallthrough marker: the
                // clause shares the next non-`-` handler's body (like `switch`).
                // Treat it as an empty body rather than lowering `-` as a script
                // — otherwise it compiles to a zero-arg call of the `-` command
                // and trips a spurious arity error (issue #703).
                //
                // Tcl recognises the marker by the word's *string value*, so the
                // braced `{-}`, quoted `"-"`, and backslash-escaped (`\-`,
                // `\x2d`, …) forms — all of which evaluate to `-` — are equally
                // fallthroughs. Braces suppress backslash substitution, so a
                // braced word's value is its raw content (`{\-}` is the literal
                // two-char string `\-`, *not* a fallthrough); bare and quoted
                // words are backslash-substituted first. A braced single-token
                // word's representative token is a `Str` (the `{`-stripping
                // wrapper kind); bare / quoted words are `Esc`.
                let is_braced = handler_tok.is_some_and(|t| t.kind == TokenType::Str);
                let body_value = if is_braced {
                    std::borrow::Cow::Borrowed(args[i + 3].as_str())
                } else {
                    tcl_lexer::backslash_subst(&args[i + 3])
                };
                let is_fallthrough = handler_single && body_value == "-";
                let handler_body = if is_fallthrough {
                    crate::ir::Script::new()
                } else {
                    self.lower_body_from_tok(&args[i + 3], handler_tok, namespace)
                };

                handlers.push(TryHandler {
                    kind: keyword.clone(),
                    match_arg,
                    var_name: result_var,
                    options_var,
                    body: handler_body,
                    body_span: handler_tok.map_or(seg.span, |t| t.span),
                    fallthrough: is_fallthrough,
                });
                i += 4;
                continue;
            }

            return Self::barrier(seg, "malformed try handler");
        }

        Statement::Try {
            span: seg.span,
            body,
            body_span: arg_tokens[0].span,
            handlers,
            finally_body,
            finally_span,
            raw_args: args.to_vec(),
        }
    }

    // switch

    /// Lower `switch ?options? subject pattern body ...`.
    // Sequential `switch` lowering: option parsing, list-form
    // unpacking, body recursion, and case-list build all share
    // local arena state.
    /// Build the `(arms, default_body, default_span)` triple from
    /// collected `SwitchPair` entries.  Extracted from
    /// [`Self::lower_switch`] to keep the dispatcher under threshold.
    fn build_switch_arms(
        &mut self,
        pairs: &[SwitchPair],
        arg_tokens: &[tcl_lexer::Token],
        namespace: &str,
    ) -> (Vec<SwitchArm>, Option<crate::ir::Script>, Option<Span>) {
        let mut arms = Vec::new();
        let mut default_body = None;
        let mut default_span = None;

        for (pair_idx, pair) in pairs.iter().enumerate() {
            // A braced multi-line pattern collapses its `\<newline>`
            // continuations to a single space, like every other braced word
            // (`switch "a b" { {a\<nl>b} … }` must match). A no-op for the
            // common single-line pattern (and for `default` / `-`).
            let pattern =
                tcl_syntax::backslash::collapse_brace_continuations_str(&pair.pattern).into_owned();
            if pair.body_text == "-" {
                arms.push(SwitchArm {
                    pattern,
                    pattern_span: pair.pattern_span,
                    body: None,
                    body_span: None,
                    fallthrough: true,
                });
                continue;
            }

            let body_tok = pair.body_arg_idx.and_then(|idx| arg_tokens.get(idx));
            let body = if let Some(tok) = body_tok {
                self.lower_body_from_tok(&pair.body_text, Some(tok), namespace)
            } else if let Some(bspan) = pair.body_span {
                // The single-braced form
                // (`switch $x { a {body} … }`) has no arg token — the
                // body tokens live inside the braced word. Lower from
                // the body's source span instead of returning an empty
                // script. `body_span` is brace-inclusive (the element
                // parser extends it over the closing delimiter), so the
                // content starts after any leading `{` / `"`; the
                // even-sized delimiter difference recovers that shift
                // and matches the offset `lower_body_from_tok` would
                // have computed from a token's `content_offset`.
                let span_len = (bspan.end().saturating_sub(bspan.start())) as usize;
                let skip = span_len.saturating_sub(pair.body_text.len()) / 2;
                let base = bspan
                    .start()
                    .saturating_add(u32::try_from(skip).unwrap_or(0));
                self.lower_body(&pair.body_text, base, namespace)
            } else {
                crate::ir::Script::new()
            };

            if pattern == "default" && pair_idx == pairs.len() - 1 {
                default_body = Some(body);
                default_span = pair.body_span;
            } else {
                arms.push(SwitchArm {
                    pattern,
                    pattern_span: pair.pattern_span,
                    body: Some(body),
                    body_span: pair.body_span,
                    fallthrough: false,
                });
            }
        }
        (arms, default_body, default_span)
    }

    pub(super) fn lower_switch(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();

        if args.len() < 2 {
            return Self::barrier(seg, "malformed switch");
        }

        let (mut i, mode, nocase, unknown) = parse_switch_options(args);

        // An unrecognised / arg-taking option (`-foo`, `-matchvar`, …): bail to
        // the runtime `switch`, which validates options and does the var writes.
        if unknown {
            return Self::barrier(seg, "switch with non-inlined option");
        }
        if i >= args.len() {
            return Self::barrier(seg, "malformed switch options");
        }

        let subject = args[i].clone();
        i += 1;
        if i >= args.len() {
            return Self::barrier(seg, "switch missing arms");
        }

        // Collect pattern/body pairs. Each pair carries a
        // `body_span` resolved in the outer source's offset
        // space plus an optional `body_arg_idx` into
        // `arg_tokens` so the body lowerer can still pick up
        // the right content offset / encoding flags.
        let mut pairs: Vec<SwitchPair> = Vec::new();

        // Patterns from a single braced body are literal list elements;
        // patterns supplied as separate words undergo runtime
        // `$var` / `[cmd]` substitution (the `switch $s $pat {body}`
        // wrapper form).
        let mut patterns_braced = true;

        // Single braced body form: switch subject { pat1 body1 pat2 body2 ... }
        if i == args.len() - 1 && i < arg_single.len() && arg_single[i] {
            let body_text = &args[i];
            // Starting offset of the body *content* inside the
            // outer source. For a braced word the content begins
            // one byte after the opening `{`.
            let outer_arg_span = arg_tokens.get(i).map_or(seg.span, |t| t.span);
            let content_shift = 1_u32; // skip leading `{`
            let body_base = outer_arg_span.start().saturating_add(content_shift);

            let elements = switch_body_elements(body_text);
            // An empty arm list (`switch x {}`) is a "wrong # args" error, not a
            // no-op — bail to the runtime command, which reports it.
            if elements.is_empty() {
                return Self::barrier(seg, "switch with no arms");
            }
            if !elements.len().is_multiple_of(2) {
                return Self::barrier(seg, "switch odd pattern count");
            }
            let relocate = |local: Span| {
                let start = body_base.saturating_add(local.start());
                let end = body_base.saturating_add(local.end());
                Span::new(start, end)
            };
            let mut j = 0;
            while j + 1 < elements.len() {
                let (pat_text, pat_local) = &elements[j];
                let (body_text_e, body_local) = &elements[j + 1];
                pairs.push(SwitchPair {
                    pattern: pat_text.clone(),
                    pattern_span: relocate(*pat_local),
                    body_text: body_text_e.clone(),
                    body_span: Some(relocate(*body_local)),
                    body_arg_idx: None,
                });
                j += 2;
            }
        } else {
            // Multi-arg form: remaining args are pattern body pairs —
            // each pattern word substitutes at runtime.
            patterns_braced = false;
            let remaining = args.len() - i;
            if !remaining.is_multiple_of(2) {
                return Self::barrier(seg, "switch odd pattern count");
            }
            while i + 1 < args.len() {
                let pattern = args[i].clone();
                let pattern_span = arg_tokens.get(i).map_or(seg.span, |t| t.span);
                let body_text_inner = args[i + 1].clone();
                let body_tok_idx = i + 1;
                // Like if/while/for/catch/try, an arm body that carries
                // substitution (`$handler`, `[cmd]`, a quoted/concatenated
                // word) is evaluated as a script from its *runtime value*, not
                // its unsubstituted spelling. Lowering the literal spelling as a
                // nested script would fabricate a phantom command (e.g. a Call
                // to `${handler}`) that downstream dead-code/taint/def-use/
                // call-graph passes reason about. Defer the whole switch to the
                // runtime command instead. The `-` fallthrough marker is a
                // literal, not a body, so it is exempt. (`seg.argv` includes the
                // command word, so the body word `args[i + 1]` is index
                // `i + 2`.) RUST_ISSUE_071.
                if body_text_inner != "-" && !super::seg_word_is_static_literal(seg, i + 2) {
                    return Self::barrier(seg, "switch with non-literal arm body");
                }
                let body_span_val = arg_tokens.get(body_tok_idx).map(|t| t.span);
                pairs.push(SwitchPair {
                    pattern,
                    pattern_span,
                    body_text: body_text_inner,
                    body_span: body_span_val,
                    body_arg_idx: Some(body_tok_idx),
                });
                i += 2;
            }
        }

        let (arms, default_body, default_span) =
            self.build_switch_arms(&pairs, arg_tokens, namespace);

        Statement::Switch {
            span: seg.span,
            subject,
            subject_span: arg_tokens.first().map_or(seg.span, |t| t.span),
            arms,
            default_body,
            default_span,
            mode,
            nocase,
            raw_args: args.to_vec(),
            patterns_braced,
        }
    }

    // dict

    /// True when this `dict` invocation's subcommand writes the dict
    /// variable at `args[1]` — the registry's per-subcommand `VarWrite`
    /// role, resolved against the actual argument list.
    fn dict_sub_writes_var(&self, args: &[String]) -> bool {
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        self.registry
            .arg_indices_for_role("dict", &arg_strs, tcl_registry::prelude::ArgRole::VarWrite)
            .contains(&1)
    }

    /// Lower `dict` subcommands.
    pub(super) fn lower_dict(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();
        let sub = &args[0];
        let sub_args = &args[1..];

        match sub.as_str() {
            "for" | "map" if sub_args.len() >= 3 => {
                let var_names = parse_param_names(&sub_args[0]);
                let body_idx = 3; // index in original args
                let body_tok = arg_tokens.get(body_idx);
                if body_tok.is_none() || body_idx >= arg_single.len() || !arg_single[body_idx] {
                    return Self::barrier(seg, &format!("dict {sub} with dynamic body"));
                }
                let body = self.lower_body_from_tok(&sub_args[2], body_tok, namespace);

                Statement::Foreach {
                    span: seg.span,
                    iterators: vec![ForeachIterator {
                        vars: var_names,
                        list_arg: sub_args[1].clone(),
                    }],
                    body,
                    body_span: body_tok.map_or(seg.span, |t| t.span),
                    is_lmap: sub == "map",
                    raw_args: args.to_vec(),
                    is_dict_iteration: true,
                    is_array_iteration: false,
                    raw_tokens: Some(Self::cmd_tokens(seg)),
                }
            }

            // The body-carrying mutators barrier (the body's local-var
            // mapping is runtime behaviour) — ordered before the generic
            // sub-mutator arm below, whose registry query also matches
            // their `VarWrite` role.
            "update" | "with" => Statement::Barrier {
                span: seg.span,
                reason: format!("dict {sub}"),
                command: seg.name().into(),
                canonical_command: None,
                args: args.to_vec(),
                tokens: Some(Self::cmd_tokens(seg)),
            },

            // A sub-mutator (`dict set/unset/append/lappend/incr …`) tags
            // the lowered call with the dict variable as a def.  Membership
            // comes from the registry's per-subcommand `VarWrite` role on
            // `dict` — the variable is `args[1]` (just past the subcommand
            // word) — never a hardcoded subcommand list.
            _ if !sub_args.is_empty() && self.dict_sub_writes_var(args) => {
                let var_name = normalise_var_name(&sub_args[0]).to_owned();
                Statement::Call {
                    span: seg.span,
                    command: seg.name().into(),
                    canonical_command: None,
                    args: args.to_vec(),
                    defs: vec![var_name],
                    reads: vec![],
                    reads_own_defs: true,
                    safe_on_uninit: false,
                    tokens: Some(Self::cmd_tokens(seg)),
                    foreach_groups: None,
                }
            }

            _ => Statement::Call {
                span: seg.span,
                command: seg.name().into(),
                canonical_command: None,
                args: args.to_vec(),
                defs: vec![],
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: Some(Self::cmd_tokens(seg)),
                foreach_groups: None,
            },
        }
    }

    // Helpers

    /// Create a barrier statement from a segmented command.
    fn barrier(seg: &SegmentedCommand, reason: &str) -> Statement {
        Statement::Barrier {
            span: seg.span,
            reason: reason.into(),
            command: seg.name().into(),
            canonical_command: None,
            args: seg.args().to_vec(),
            tokens: Some(Self::cmd_tokens(seg)),
        }
    }

    /// Lower a body argument using token offset info.
    pub(super) fn lower_body_from_tok(
        &mut self,
        text: &str,
        tok: Option<&tcl_lexer::Token>,
        namespace: &str,
    ) -> Script {
        let Some(tok) = tok else {
            return Script::new();
        };
        let offset = tok.span.start() + u32::from(tok.content_offset);
        self.lower_body(text, offset, namespace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowering::lower_to_ir;
    use tcl_registry::CommandRegistry;

    fn reg() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    #[test]
    fn switch_exact() {
        // Both forms lower to a structured Switch; the multi-arg form
        // and the single-braced form must agree on shape.
        for src in [
            "switch $x { a {puts a} b {puts b} }",
            "switch $x a {puts a} b {puts b}",
        ] {
            let m = lower_to_ir(src, &reg());
            assert!(
                matches!(
                    &m.top_level.statements[0],
                    Statement::Switch { mode, .. } if *mode == SwitchMode::Exact
                ),
                "expected exact Switch for {src:?}",
            );
        }
    }

    #[test]
    fn switch_single_braced_body_is_lowered() {
        // The single-braced arm form
        // `switch $x { a {body} … }` must lower each arm body into real
        // IR statements (it used to produce an empty Script).
        let m = lower_to_ir("switch $x { a {puts hi} b {set y 1} }", &reg());
        let Statement::Switch { arms, .. } = &m.top_level.statements[0] else {
            panic!("expected Switch");
        };
        assert_eq!(arms.len(), 2);
        for arm in arms {
            let body = arm.body.as_ref().expect("arm body");
            assert!(
                !body.statements.is_empty(),
                "single-braced arm body should lower to non-empty IR, got {body:?}",
            );
        }
    }

    #[test]
    fn switch_substituted_arm_body_barriers() {
        // RUST_ISSUE_071: a multi-arg arm body that is a substitution
        // (`$handler`, `[cmd]`, a quoted word) must NOT be lowered from its
        // unsubstituted spelling — the switch defers to the runtime command so
        // no phantom Call to `${handler}` is fabricated.
        for src in [
            "switch $x a $handler",
            "switch $x a [get_handler]",
            "switch $x a \"puts $x\"",
            "switch $x a {puts a} b $other",
        ] {
            let m = lower_to_ir(src, &reg());
            assert!(
                matches!(&m.top_level.statements[0], Statement::Barrier { .. }),
                "expected Barrier (defer to runtime) for {src:?}, got {:?}",
                m.top_level.statements[0],
            );
        }
    }

    #[test]
    fn switch_static_arm_body_still_lowers() {
        // FP-guard for RUST_ISSUE_071: genuinely literal arm bodies (braced,
        // and the `-` fallthrough marker) still lower to a structured Switch.
        for src in [
            "switch $x a {puts a} b {puts b}",
            "switch $x a - b {puts b}",
        ] {
            let m = lower_to_ir(src, &reg());
            assert!(
                matches!(&m.top_level.statements[0], Statement::Switch { .. }),
                "expected structured Switch for {src:?}, got {:?}",
                m.top_level.statements[0],
            );
        }
    }

    // `patterns_braced` distinguishes a literal-pattern
    // braced block from the substituting separate-words form.

    #[test]
    fn switch_braced_block_sets_patterns_braced_true() {
        let m = lower_to_ir("switch $x { a {puts hi} b {set y 1} }", &reg());
        let Statement::Switch {
            patterns_braced, ..
        } = &m.top_level.statements[0]
        else {
            panic!("expected Switch");
        };
        assert!(
            *patterns_braced,
            "single braced `{{pat body …}}` block → patterns_braced = true",
        );
    }

    #[test]
    fn switch_separate_words_sets_patterns_braced_false() {
        // `switch $s a {body1} b {body2}` — patterns are separate words
        // that substitute at runtime, so patterns_braced must be false.
        let m = lower_to_ir("switch $x a {puts hi} b {set y 1}", &reg());
        let Statement::Switch {
            patterns_braced, ..
        } = &m.top_level.statements[0]
        else {
            panic!("expected Switch");
        };
        assert!(
            !*patterns_braced,
            "separate-words pattern form → patterns_braced = false",
        );
    }

    #[test]
    fn try_with_finally() {
        let m = lower_to_ir("try {error oops} finally {cleanup}", &reg());
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Try {
                finally_body: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn try_with_handler() {
        let m = lower_to_ir("try {error oops} on error {e opts} {puts $e}", &reg());
        if let Statement::Try { handlers, .. } = &m.top_level.statements[0] {
            assert_eq!(handlers.len(), 1);
            assert_eq!(handlers[0].kind, "on");
        } else {
            panic!("expected Try");
        }
    }

    #[test]
    fn try_dash_handler_body_is_fallthrough() {
        // Issue #703: a `-` handler body is a fallthrough marker (shares the
        // next non-`-` handler's body, like `switch`), not a zero-arg `-`
        // command. It must lower to a fallthrough handler with an empty body.
        let m = lower_to_ir(
            "try {set x 1} on ok result - trap NONE result {return 0} on error msg {return 1}",
            &reg(),
        );
        let Statement::Try { handlers, .. } = &m.top_level.statements[0] else {
            panic!("expected Try");
        };
        let shape: Vec<(&str, &str, bool)> = handlers
            .iter()
            .map(|h| (h.kind.as_str(), h.match_arg.as_str(), h.fallthrough))
            .collect();
        assert_eq!(
            shape,
            vec![
                ("on", "ok", true),
                ("trap", "NONE", false),
                ("on", "error", false)
            ],
        );
        // The fallthrough handler carries no statements of its own.
        assert!(handlers[0].body.statements.is_empty());
        assert!(!handlers[1].body.statements.is_empty());
    }

    #[test]
    fn try_braced_dash_handler_body_is_fallthrough() {
        // Per Tcl's `TclNRTryObjCmd`, a body of `{-}` evaluates to the string
        // `-` and is equally a fallthrough marker.
        let m = lower_to_ir("try {set x 1} on ok a {-} trap NONE b {return $b}", &reg());
        let Statement::Try { handlers, .. } = &m.top_level.statements[0] else {
            panic!("expected Try");
        };
        assert!(handlers[0].fallthrough);
        assert!(handlers[0].body.statements.is_empty());
    }

    #[test]
    fn try_consecutive_dash_handlers_are_fallthrough() {
        // Several `-` handlers in a row all share the final body.
        let m = lower_to_ir(
            "try {set x 1} on ok a - on return b - trap NONE c {return $c}",
            &reg(),
        );
        let Statement::Try { handlers, .. } = &m.top_level.statements[0] else {
            panic!("expected Try");
        };
        let flags: Vec<bool> = handlers.iter().map(|h| h.fallthrough).collect();
        assert_eq!(flags, vec![true, true, false]);
    }

    #[test]
    fn try_empty_brace_handler_body_is_not_fallthrough() {
        // A genuinely empty `{}` body is valid and distinct from `-`.
        let m = lower_to_ir("try {set x 1} on ok a {} on error b {return $b}", &reg());
        let Statement::Try { handlers, .. } = &m.top_level.statements[0] else {
            panic!("expected Try");
        };
        assert!(!handlers[0].fallthrough);
    }

    #[test]
    fn try_quoted_dash_handler_body_is_fallthrough() {
        // A quoted `"-"` evaluates to the string `-`, like the bare/braced forms.
        let m = lower_to_ir(
            "try {set x 1} on ok a \"-\" trap NONE b {return $b}",
            &reg(),
        );
        let Statement::Try { handlers, .. } = &m.top_level.statements[0] else {
            panic!("expected Try");
        };
        assert!(handlers[0].fallthrough);
        assert!(handlers[0].body.statements.is_empty());
    }

    #[test]
    fn try_backslash_escaped_dash_handler_body_is_fallthrough() {
        // Tcl applies backslash substitution before `try` sees the word, so a
        // bare `\-` / `\x2d` body evaluates to `-` and is a fallthrough
        // (Codex review on #706 / port of #704).
        for src in [
            "try {set x 1} on ok a \\- trap NONE b {return $b}",
            "try {set x 1} on ok a \\x2d trap NONE b {return $b}",
            "try {set x 1} on ok a \"\\-\" trap NONE b {return $b}",
        ] {
            let m = lower_to_ir(src, &reg());
            let Statement::Try { handlers, .. } = &m.top_level.statements[0] else {
                panic!("expected Try for {src:?}");
            };
            assert!(handlers[0].fallthrough, "expected fallthrough for {src:?}");
            assert!(
                handlers[0].body.statements.is_empty(),
                "expected empty body for {src:?}",
            );
        }
    }

    #[test]
    fn try_braced_escaped_dash_handler_body_is_not_fallthrough() {
        // Braces suppress backslash substitution: `{\-}` is the literal
        // two-char string `\-`, which is *not* the `-` fallthrough marker.
        let m = lower_to_ir(
            "try {set x 1} on ok a {\\-} trap NONE b {return $b}",
            &reg(),
        );
        let Statement::Try { handlers, .. } = &m.top_level.statements[0] else {
            panic!("expected Try");
        };
        assert!(
            !handlers[0].fallthrough,
            "braced `{{\\-}}` is a literal string, not a fallthrough",
        );
    }

    #[test]
    fn catch_with_vars() {
        let m = lower_to_ir("catch {expr 1/0} result opts", &reg());
        if let Statement::Catch {
            result_var,
            options_var,
            ..
        } = &m.top_level.statements[0]
        {
            assert_eq!(result_var.as_deref(), Some("result"));
            assert_eq!(options_var.as_deref(), Some("opts"));
        } else {
            panic!("expected Catch");
        }
    }

    #[test]
    fn catch_preserves_command_tokens() {
        // ``Statement::Catch`` must carry the full ``CommandTokens``
        // snapshot so the CFG's ``Catch → Call`` lowering
        // (`emit_opaque_catch`) can preserve the brace-vs-bare
        // shape of the body word when reconstructing the script
        // for the runtime's eval-fallback.
        let m = lower_to_ir("catch {set x 1} result", &reg());
        if let Statement::Catch { tokens, .. } = &m.top_level.statements[0] {
            let tokens = tokens.as_ref().expect("tokens populated");
            // 3 words: ``catch`` + ``{set x 1}`` + ``result``.
            assert_eq!(tokens.argv.len(), 3);
        } else {
            panic!("expected Catch");
        }
    }

    #[test]
    fn catch_dollar_var_body_falls_through_to_barrier() {
        // ``catch $cmd res`` has a single VAR token body, not a
        // brace-literal `Str` token.  The lowering must treat it
        // as a dynamic body and emit a Barrier so the runtime
        // ``eval_catch`` evaluates the substituted value.
        let m = lower_to_ir("catch $cmd res", &reg());
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Barrier { reason, .. } if reason == "catch with dynamic body"
        ));
    }

    #[test]
    fn catch_bracket_command_body_falls_through_to_barrier() {
        // ``catch [build] res`` — single-token but command-subst,
        // not brace-literal.  Must hit the Barrier path.
        let m = lower_to_ir("catch [build] res", &reg());
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Barrier { reason, .. } if reason == "catch with dynamic body"
        ));
    }

    #[test]
    fn dict_set() {
        let m = lower_to_ir("dict set d key value", &reg());
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Call { command, defs, .. }
                if command == "dict" && defs.contains(&"d".to_string())
        ));
    }

    #[test]
    fn dict_for() {
        let m = lower_to_ir("dict for {k v} $d {puts $k}", &reg());
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Foreach {
                is_dict_iteration: true,
                ..
            }
        ));
    }
}

#[cfg(test)]
mod switch_span_tests {
    use crate::compilation_unit::CompilationUnit;
    use crate::ir::Statement;
    use tcl_registry::CommandRegistry;

    #[test]
    fn switch_arm_and_default_spans_point_at_body_text() {
        let src = "switch foo { foo { puts one } bar { puts two } default { puts none } }";
        let r = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(src, &r, false);

        // Locate the switch statement in the IR (top-level).
        let switch = cu
            .ir_module
            .top_level
            .statements
            .iter()
            .find(|s| matches!(s, Statement::Switch { .. }))
            .expect("switch statement");

        let Statement::Switch {
            arms,
            default_body,
            default_span,
            ..
        } = switch
        else {
            panic!("expected Switch variant");
        };

        assert_eq!(arms.len(), 2);
        for arm in arms {
            let span = arm.body_span.expect("arm body_span populated");
            let text = &src[span.as_range()];
            assert!(
                text.starts_with('{') && text.ends_with('}'),
                "expected braced body, got {text:?}",
            );
            assert!(
                text.contains("puts"),
                "expected body to contain `puts`, got {text:?}",
            );
        }

        assert!(default_body.is_some());
        let dspan = default_span.expect("default_span populated");
        let dtext = &src[dspan.as_range()];
        assert!(
            dtext.contains("puts none"),
            "expected default body to contain `puts none`, got {dtext:?}",
        );
    }
}
