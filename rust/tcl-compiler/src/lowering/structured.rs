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

use tcl_lexer::{Span, Token, TokenType};

use crate::expr_parser::parse_expr_for_profile;
use crate::ir::{ForeachIterator, IfClause, Script, Statement, SwitchArm, SwitchMode, TryHandler};
use crate::lowering_hooks::word_content_base;
use crate::naming::normalise_var_name;
use crate::segmenter::SegmentedCommand;

use super::{Lowerer, parse_var_list_names};
use tcl_syntax::word_rules::WordValueRules;

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

/// One element of a switch's braced case list.
struct SwitchElement {
    /// The element's decoded **value** — what C Tcl's list split hands the
    /// pattern comparison (`Tcl_SplitList` semantics: braced content
    /// verbatim, backslashes collapsed in bare / quoted elements, so a bare
    /// `a\ b` pattern is the value `a b`).
    value: String,
    /// The element's raw interior text as written (delimiters stripped,
    /// backslashes untouched) — what a braced body's script lowering reads.
    raw: String,
    /// Local span, delimiter-inclusive for `{…}` / `"…"` elements, in the
    /// body text's own offset space — callers relocate it to the full
    /// source buffer by adding the body text's starting offset.
    span: Span,
}

/// Split a switch's braced case list into its elements with the central
/// Tcl **list** grammar ([`tcl_syntax::list::find_element`]).
///
/// A braced case list is a list, not a script: C Tcl's
/// `TclNRSwitchObjCmd` splits it with `TclListObjGetElements`
/// (`generic/tclCmdMZ.c`), so `#` starts no comment there and `;` is an
/// ordinary pattern character.  The previous script-lexer implementation
/// skipped `TokenType::Comment` tokens, silently deleting a valid `#`
/// pattern and its body (issue #1197 — tclsh 9.0.4: `switch # { #
/// {puts matched} default {puts default} }` prints `matched`).
///
/// Returns `None` when the text is not a well-formed list — the caller
/// bails the whole `switch` to the runtime command, which reports the
/// error exactly as C Tcl does.
fn switch_body_elements(body_text: &str) -> Option<Vec<SwitchElement>> {
    let bytes = body_text.as_bytes();
    let mut elements = Vec::new();
    let mut scan = 0usize;
    loop {
        match tcl_syntax::list::find_element(body_text, scan) {
            Ok(Some(el)) => {
                let raw = body_text[el.value.clone()].to_owned();
                let value = if el.literal {
                    raw.clone()
                } else {
                    tcl_lexer::backslash_subst(&raw).into_owned()
                };
                let quoted = !el.braced
                    && el.value.start > 0
                    && bytes.get(el.value.start - 1) == Some(&b'"');
                let (start, end) = if el.braced || quoted {
                    (el.value.start - 1, el.value.end + 1)
                } else {
                    (el.value.start, el.value.end)
                };
                let local_span = Span::new(
                    u32::try_from(start).unwrap_or(u32::MAX),
                    u32::try_from(end).unwrap_or(u32::MAX),
                );
                let next = el.next;
                elements.push(SwitchElement {
                    value,
                    raw,
                    span: local_span,
                });
                if next <= scan {
                    break;
                }
                scan = next;
            }
            Ok(None) => break,
            Err(_) => return None,
        }
    }
    Some(elements)
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
            return self.barrier(seg, "malformed if");
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
                    return self.barrier(seg, "if missing elseif expression");
                }
                continue;
            }
            if args[i] == "else" {
                if i + 1 >= args.len() {
                    return self.barrier(seg, "malformed if else clause");
                }
                // Exactly one body may follow `else`; trailing words
                // (`if 0 {a} else {b} junk`) are "extra words after else" — defer to
                // the runtime `if` (if-3.5).
                if i + 2 < args.len() {
                    return self.barrier(seg, "if extra words after else");
                }
                // Only a substitution-free literal body inlines (see the
                // clause-body note below).
                if !super::seg_word_is_static_literal(seg, i + 2) {
                    return self.barrier(seg, "if with non-literal body");
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
                return self.barrier(seg, "malformed if clause");
            }

            let body_idx = i;
            // C's TclCompileIfCmd only inlines a braced-literal body; a body
            // carrying substitutions (`$x`, `[cmd]`, a quoted or concatenated
            // word like `$x1$x2`) must be substituted *then* evaluated as a
            // script — which the runtime `if` command does. Bail the whole
            // construct to that command rather than mis-parsing the unsubstituted
            // word as a literal script at compile time.
            if !super::seg_word_is_static_literal(seg, body_idx + 1) {
                return self.barrier(seg, "if with non-literal body");
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
                condition: parse_expr_for_profile(&cond_text, self.dialect),
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
                return self.barrier(seg, "if with extra words");
            }
        }

        if clauses.is_empty() {
            return self.barrier(seg, "malformed if");
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
            return self.barrier(seg, "malformed for");
        }
        // `init`, `next`, and `body` are all lowered via `lower_body_from_tok`,
        // which rebases the segmenter-reconstructed word text at the token's
        // span — safe only for a substitution-free literal (`seg_word_is_static_literal`,
        // matching `TclCompileForCmd`'s `TCL_TOKEN_SIMPLE_WORD` check). A single-token
        // but dynamic word (`$body`, `[cmd]`) must barrier rather than be
        // lowered as if it were the literal text (issue #1375).
        if !arg_single[1]
            || !super::seg_word_is_static_literal(seg, 1)
            || !super::seg_word_is_static_literal(seg, 3)
            || !super::seg_word_is_static_literal(seg, 4)
        {
            return self.barrier(seg, "for with dynamic arguments");
        }
        // A body/next that redefines break/continue must run through the runtime
        // builtin, which dispatches them (so the redefinition is honoured) rather
        // than firing the compiled JUMP fast-path unconditionally and looping
        // forever (proc-7.3).
        if redefines_loop_control(&args[2]) || redefines_loop_control(&args[3]) {
            return self.barrier(seg, "for redefines break/continue");
        }

        let init = self.lower_body_from_tok(&args[0], Some(&arg_tokens[0]), namespace);
        let next = self.lower_body_from_tok(&args[2], Some(&arg_tokens[2]), namespace);
        let body = self.lower_body_from_tok(&args[3], Some(&arg_tokens[3]), namespace);

        Statement::For {
            span: seg.span,
            init,
            init_span: arg_tokens[0].span,
            condition: parse_expr_for_profile(
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
            raw_tokens: Some(self.cmd_tokens(seg)),
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
            return self.barrier(seg, "malformed while");
        }
        // The body is lowered via `lower_body_from_tok`, which rebases the
        // segmenter-reconstructed word text at the token's span — safe only
        // for a substitution-free literal (matching `TclCompileWhileCmd`'s
        // `TCL_TOKEN_SIMPLE_WORD` check on the body). A single-token but
        // dynamic body (`$body`, `[cmd]`) must barrier rather than be lowered
        // as if it were the literal text (issue #1375).
        if !arg_single[0] || !super::seg_word_is_static_literal(seg, 2) {
            return self.barrier(seg, "while with dynamic arguments");
        }
        // See `lower_for`: a body redefining break/continue must use the runtime
        // builtin so the redefinition is dispatched (proc-7.3).
        if redefines_loop_control(&args[1]) {
            return self.barrier(seg, "while redefines break/continue");
        }

        let body = self.lower_body_from_tok(&args[1], Some(&arg_tokens[1]), namespace);

        Statement::While {
            span: seg.span,
            condition: parse_expr_for_profile(
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
            raw_tokens: Some(self.cmd_tokens(seg)),
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

        if args.len() < 3 || args.len().is_multiple_of(2) {
            return self.barrier(seg, "malformed foreach");
        }

        let body_idx = args.len() - 1;
        let body_tok = arg_tokens.get(body_idx);
        // The body is lowered via `lower_body_from_tok`, which rebases the
        // segmenter-reconstructed word text at the token's span — safe only
        // for a substitution-free literal (matching `TclCompileForeachCmd`'s
        // `TCL_TOKEN_SIMPLE_WORD` check on the body). A single-token but
        // dynamic body (`$body`, `[cmd]`) must barrier rather than be lowered
        // as if it were the literal text (issue #1375).
        if body_tok.is_none() || !super::seg_word_is_static_literal(seg, body_idx + 1) {
            return self.barrier(seg, "foreach with dynamic body");
        }

        // The value word's brace-quoting is a fact only the tokens carry:
        // `foreach n {a $b c}` iterates the three literal elements `a`,
        // `$b`, `c` and reads nothing (tclsh 8.6.14), while `foreach n
        // "a $b c"` substitutes. Both lower to the same `list_arg` text,
        // so the flag is what downstream read-harvesting gates on
        // (issue #1260).
        let cmd_tokens = self.cmd_tokens(seg);
        let mut iterators = Vec::new();
        for i in (0..body_idx).step_by(2) {
            // A varList that is not a well-formed Tcl list binds nothing we can
            // name; the runtime `foreach` raises Tcl's own error.
            let Some(var_names) =
                parse_var_list_names(&args[i], WordValueRules::from_config(&self.config))
            else {
                return self.barrier(seg, "foreach with malformed variable list");
            };
            iterators.push(ForeachIterator {
                vars: var_names,
                list_arg: args[i + 1].clone(),
                list_braced: cmd_tokens.arg_is_braced_literal(i + 1),
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
        //     `lmap` boundary, whose explicit activation preserves `yield` and
        //     non-local completions. A straight-line `lmap` lowers inline —
        //     yieldable and correctly collecting.
        //   * a `foreach` whose body directly contains another `foreach` or
        //     `lmap`. Inline iterator state is activation-local, but the CFG
        //     layout cannot yet pair the outer step with a nested loop's
        //     back-edge. Routing the outer loop through the runtime command
        //     gives the nested body a fresh activation, preserving both
        //     iterator stacks and non-local loop completions.
        // The runtime builtin evaluates the body transparently. It remains the
        // correct fallback for a collecting loop with branching control flow,
        // whose per-arm result collection cannot be represented by the single
        // inline `LMAP_COLLECT` tail, and for nested iterator layouts pending a
        // dedicated CFG representation.
        let body_nests_foreach = body
            .statements
            .iter()
            .any(|statement| matches!(statement, Statement::Foreach { .. }));
        let lmap_needs_runtime = is_lmap && !Self::body_is_straight_line(&body);
        if self.target.is_bytecode() && (lmap_needs_runtime || body_nests_foreach) {
            return self.barrier(seg, if is_lmap { "lmap" } else { "foreach" });
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
            raw_tokens: Some(cmd_tokens),
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
    fn body_is_straight_line(body: &Script) -> bool {
        body.statements.iter().all(|s| {
            matches!(
                s,
                Statement::Call { .. }
                    | Statement::AssignConst { .. }
                    | Statement::AssignExpr { .. }
                    | Statement::AssignValue { .. }
                    | Statement::Incr { .. }
                    | Statement::ExprEval { .. }
                    | Statement::Barrier { .. }
            )
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

        // `foreachLine varName filename body` — exactly three args.
        if args.len() != 3 {
            return self.barrier(seg, "malformed foreachLine");
        }

        // Body must be a single static brace-string literal; dynamic
        // bodies (`$body`, `[cmd]`, multi-token) fall through to the
        // runtime command via `Statement::Barrier`.  Mirrors the
        // `lower_catch` body guard — a `Var` / `Cmd` single-token
        // word is still dynamic and must not be compiled as a
        // static loop body.
        let body_tok = arg_tokens.get(2);
        if !super::seg_word_is_static_braced(seg, 3) {
            return self.barrier(seg, "foreachLine with dynamic body");
        }

        // Single iterator binding the loop variable.  `list_arg`
        // semantically carries "the iteration source" — for plain
        // `foreach` that's the list; for `foreachLine` it's the
        // filename (the runtime reads lines from it).  Downstream
        // dataflow doesn't care: the lattice-propagation matters,
        // not the literal value.  See the type-level doc-comment
        // above for the runtime-semantics caveat.
        let cmd_tokens = self.cmd_tokens(seg);
        let Some(vars) = parse_var_list_names(&args[0], WordValueRules::from_config(&self.config))
        else {
            return self.barrier(seg, "foreachLine with malformed variable list");
        };
        let iterators = vec![ForeachIterator {
            vars,
            list_arg: args[1].clone(),
            list_braced: cmd_tokens.arg_is_braced_literal(1),
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
            raw_tokens: Some(cmd_tokens),
        }
    }

    // catch

    /// Lower `catch body ?resultVar? ?optionsVar?`.
    pub(super) fn lower_catch(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();

        if args.is_empty() {
            return self.barrier(seg, "malformed catch");
        }
        // Body must be a single brace-literal (`Str` kind) token to
        // compile statically.  Variable references (`$cmd`) and
        // bracket commands (`[expr ...]`) are single-token but
        // non-`Str` and must fall through to the runtime
        // `eval_catch`, which calls `eval_script` on the substituted
        // value.  Without the kind check, ``catch $cmd res`` would
        // be compiled as "call the proc named by ``$cmd``" — wrong.
        if arg_tokens.is_empty() || !super::seg_word_is_static_braced(seg, 1) {
            return self.barrier(seg, "catch with dynamic body");
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
            tokens: Some(self.cmd_tokens(seg)),
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
            return self.barrier(seg, "try");
        }

        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();

        if args.is_empty() {
            return self.barrier(seg, "malformed try");
        }
        // The body is lowered via `lower_body_from_tok`, which rebases the
        // segmenter-reconstructed word text at the token's span — safe only
        // for a brace-literal (`Str`) token, matching `lower_catch`'s body
        // guard. A single-token but dynamic body (`$body`, `[cmd]`) must
        // barrier rather than be lowered as if it were the literal text
        // (issue #1375).
        if arg_tokens.is_empty() || !super::seg_word_is_static_braced(seg, 1) {
            return self.barrier(seg, "try with dynamic body");
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
                // The `finally` word is a body like any other, so it needs the
                // same static gate as the primary body above: single-token-ness
                // alone let `try {} finally $body` through, and rebasing the
                // reconstructed `${body}` text at the `$body` token's span put
                // the inner statement off the end of the source (issue #1375,
                // missed here; PR #1481 review).  C's `TclCompileTryCmd` makes
                // the same call — a `finally` word that is not a
                // `TCL_TOKEN_SIMPLE_WORD` is `goto failedToCompile`, deferring
                // the whole `try` to the runtime command.
                if fin_tok.is_none() || !super::seg_word_is_static_braced(seg, i + 2) {
                    return self.barrier(seg, "try with dynamic finally body");
                }
                finally_body = Some(self.lower_body_from_tok(&args[i + 1], fin_tok, namespace));
                finally_span = fin_tok.map(|t| t.span);
                i += 2;
                continue;
            }

            if (keyword == "on" || keyword == "trap") && i + 3 < args.len() {
                let match_arg = args[i + 1].clone();
                // A trap selector is evaluated as one Tcl word and its value
                // is then parsed as a Tcl list. Preserve the source-level
                // substitution decision here: inspecting the decoded elements
                // for `$` or `[` would reject literal data such as `{A {$B}}`
                // and `{A \$B}`. A substitution-free bare/quoted word still
                // needs backslash substitution before list parsing, whereas a
                // braced word's content is already its literal runtime value.
                let trap_pattern =
                    if keyword == "trap" && super::seg_word_is_static_literal(seg, i + 2) {
                        let match_tok = arg_tokens.get(i + 1);
                        let value = if match_tok.is_some_and(|tok| tok.kind == TokenType::Str) {
                            std::borrow::Cow::Borrowed(match_arg.as_str())
                        } else {
                            tcl_lexer::backslash_subst_in(&match_arg, self.config.escapes)
                        };
                        WordValueRules::from_config(&self.config)
                            .split_list(&value)
                            .ok()
                            .map(|elements| elements.into_iter().map(Into::into).collect())
                    } else {
                        None
                    };
                let var_list = &args[i + 2];
                let handler_tok = arg_tokens.get(i + 3);
                let handler_single = arg_single.get(i + 3).copied().unwrap_or(false);

                let Some(var_names) =
                    parse_var_list_names(var_list, WordValueRules::from_config(&self.config))
                else {
                    return self.barrier(seg, "try with malformed handler variable list");
                };
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
                    tcl_lexer::backslash_subst_in(&args[i + 3], self.config.escapes)
                };
                let is_fallthrough = handler_single && body_value == "-";
                // Every handler body that is *not* the fallthrough marker gets
                // the primary body's static gate: `on error {} $body` is a
                // single VAR token, so the old unconditional walk rebased the
                // reconstructed `${body}` text at that token and emitted a span
                // past the end of the source (issue #1375, missed here; PR
                // #1481 review).  C's `TclCompileTryCmd` refuses the same shape
                // — a handler body that is not a `TCL_TOKEN_SIMPLE_WORD` is
                // `goto failedToCompile` — so the whole `try` defers to the
                // runtime command, exactly as the primary-body gate does.
                if !is_fallthrough
                    && (handler_tok.is_none() || !super::seg_word_is_static_braced(seg, i + 4))
                {
                    return self.barrier(seg, "try with dynamic handler body");
                }
                let handler_body = if is_fallthrough {
                    crate::ir::Script::new()
                } else {
                    self.lower_body_from_tok(&args[i + 3], handler_tok, namespace)
                };

                handlers.push(TryHandler {
                    kind: keyword.clone(),
                    match_arg,
                    trap_pattern,
                    var_name: result_var,
                    options_var,
                    body: handler_body,
                    body_span: handler_tok.map_or(seg.span, |t| t.span),
                    fallthrough: is_fallthrough,
                });
                i += 4;
                continue;
            }

            return self.barrier(seg, "malformed try handler");
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
            let pattern = WordValueRules::from_config(&self.config)
                .collapse_braced_word(&pair.pattern)
                .into_owned();
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
            return self.barrier(seg, "malformed switch");
        }

        let (mut i, mode, nocase, unknown) = parse_switch_options(args);

        // An unrecognised / arg-taking option (`-foo`, `-matchvar`, …): bail to
        // the runtime `switch`, which validates options and does the var writes.
        if unknown {
            return self.barrier(seg, "switch with non-inlined option");
        }
        if i >= args.len() {
            return self.barrier(seg, "malformed switch options");
        }

        let subject = args[i].clone();
        // Whether the subject word was *braced*, which decides whether its
        // value is literal. `TokenType::Str` is the braced word plus a
        // single-token check, which is the predicate the rest of lowering uses
        // for this question (`braced_at` in `lowering::mod`). `arg_single`
        // alone is not it — it means "one token", which `$x` and a bare word
        // also satisfy, and bracing those would freeze a subject that must
        // substitute.
        let subject_braced = matches!(
            arg_tokens.get(i).map(|t| t.kind),
            Some(tcl_lexer::TokenType::Str)
        ) && arg_single.get(i).copied().unwrap_or(false);
        i += 1;
        if i >= args.len() {
            return self.barrier(seg, "switch missing arms");
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

            // Not a well-formed Tcl list — bail to the runtime `switch`,
            // which reports the list error exactly as C Tcl does.
            let Some(elements) = switch_body_elements(body_text) else {
                return self.barrier(seg, "switch case list is not a list");
            };
            // An empty arm list (`switch x {}`) is a "wrong # args" error, not a
            // no-op — bail to the runtime command, which reports it.
            if elements.is_empty() {
                return self.barrier(seg, "switch with no arms");
            }
            if !elements.len().is_multiple_of(2) {
                return self.barrier(seg, "switch odd pattern count");
            }
            let relocate = |local: Span| {
                let start = body_base.saturating_add(local.start());
                let end = body_base.saturating_add(local.end());
                Span::new(start, end)
            };
            let mut j = 0;
            while j + 1 < elements.len() {
                let pat = &elements[j];
                let body = &elements[j + 1];
                pairs.push(SwitchPair {
                    // The pattern is the element's decoded list VALUE
                    // (`a\ b` matches the subject `a b`); the body keeps
                    // its raw spelling for script lowering.
                    pattern: pat.value.clone(),
                    pattern_span: relocate(pat.span),
                    body_text: body.raw.clone(),
                    body_span: Some(relocate(body.span)),
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
                return self.barrier(seg, "switch odd pattern count");
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
                // `i + 2`.).
                if body_text_inner != "-" && !super::seg_word_is_static_literal(seg, i + 2) {
                    return self.barrier(seg, "switch with non-literal arm body");
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
            subject_braced,
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
        let sub = &args[0];
        let sub_args = &args[1..];

        match sub.as_str() {
            "for" | "map" if sub_args.len() >= 3 => {
                let Some(var_names) =
                    parse_var_list_names(&sub_args[0], WordValueRules::from_config(&self.config))
                else {
                    return self.barrier(seg, &format!("dict {sub} with malformed variable list"));
                };
                let body_idx = 3; // index in original args
                let body_tok = arg_tokens.get(body_idx);
                // The body is lowered via `lower_body_from_tok`, which
                // rebases the segmenter-reconstructed word text at the
                // token's span — safe only for a brace-literal (`Str`)
                // token, matching `lower_catch`'s body guard. A
                // single-token but dynamic body (`$body`, `[cmd]`) must
                // barrier rather than be lowered as if it were the literal
                // text (issue #1375).
                if body_tok.is_none() || !super::seg_word_is_static_braced(seg, body_idx + 1) {
                    return self.barrier(seg, &format!("dict {sub} with dynamic body"));
                }
                let body = self.lower_body_from_tok(&sub_args[2], body_tok, namespace);

                Statement::Foreach {
                    span: seg.span,
                    iterators: vec![ForeachIterator {
                        vars: var_names,
                        list_arg: sub_args[1].clone(),
                        // `dict for {k v} {a $b} …` iterates the literal
                        // dictionary; the value word is `args[2]` in the
                        // full argument list (issue #1260).
                        list_braced: self.cmd_tokens(seg).arg_is_braced_literal(2),
                    }],
                    body,
                    body_span: body_tok.map_or(seg.span, |t| t.span),
                    is_lmap: sub == "map",
                    raw_args: args.to_vec(),
                    is_dict_iteration: true,
                    is_array_iteration: false,
                    raw_tokens: Some(self.cmd_tokens(seg)),
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
                tokens: Some(self.cmd_tokens(seg)),
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
                    safe_on_uninit: self.safe_on_uninit(seg.name(), args),
                    tokens: Some(self.cmd_tokens(seg)),
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
                tokens: Some(self.cmd_tokens(seg)),
                foreach_groups: None,
            },
        }
    }

    // Helpers

    /// Create a barrier statement from a segmented command.
    fn barrier(&self, seg: &SegmentedCommand, reason: &str) -> Statement {
        Statement::Barrier {
            span: seg.span,
            reason: reason.into(),
            command: seg.name().into(),
            canonical_command: None,
            args: seg.args().to_vec(),
            tokens: Some(self.cmd_tokens(seg)),
        }
    }

    /// Lower a body argument using token offset info.
    ///
    /// Rebasing the body's spans by one offset is truthful only while `text`
    /// maps 1:1 onto the source region `tok` covers.  A compound `{body}x`
    /// word does not: its value is the brace content welded to the trailing
    /// fragment with the `}` dropped, so every token past the drop slides one
    /// byte left — an off-by-one span on ASCII, an offset inside a UTF-8
    /// sequence on anything else (issue #1325).  Clamp to the part that does
    /// map, exactly as the analyser's `analyse_body` does, so the welded tail
    /// — which is not a script in the first place — is dropped rather than
    /// lowered at fictional offsets.  An ordinary braced body fills its
    /// region and passes through untouched.
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
        let text = self.guarded_body_text(*tok, text);
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
        // A multi-arg arm body that is a substitution
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
        // FP-guard for: genuinely literal arm bodies (braced,
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
    fn try_trap_pattern_projection_preserves_tcl_word_literalness() {
        let source = r#"try {} \
            trap {A \$B} {m o} {} \
            trap {A {$B}} {m o} {} \
            trap {A \[B\]} {m o} {} \
            trap {A {[B]}} {m o} {} \
            trap {A C:\\tmp} {m o} {} \
            trap {A {C:\tmp}} {m o} {} \
            trap "A \{" {m o} {} \
            trap $pattern {m o} {} \
            trap "A $pattern" {m o} {} \
            trap [list A B] {m o} {}"#;
        let module = lower_to_ir(source, &reg());
        let Statement::Try { handlers, .. } = &module.top_level.statements[0] else {
            panic!("expected structured try");
        };
        let patterns: Vec<_> = handlers
            .iter()
            .map(|handler| handler.trap_pattern.clone())
            .collect();
        assert_eq!(
            patterns,
            vec![
                Some(vec!["A".into(), "$B".into()]),
                Some(vec!["A".into(), "$B".into()]),
                Some(vec!["A".into(), "[B]".into()]),
                Some(vec!["A".into(), "[B]".into()]),
                Some(vec!["A".into(), r"C:\tmp".into()]),
                Some(vec!["A".into(), r"C:\tmp".into()]),
                None,
                None,
                None,
                None,
            ]
        );
    }

    #[test]
    fn try_trap_pattern_projection_uses_the_release_escape_grammar() {
        // Tcl 8.6 consumes the same wide-unicode escape extent as Tcl 9,
        // but its UTF-16-internal build replaces a non-BMP scalar with
        // U+FFFD. Tcl 9 retains the scalar. This is observable in trap's
        // error-code prefix and therefore must follow the lowerer's profile.
        let source = r#"try {} trap "A \U0001F600" {m o} {}"#;
        let pattern = |dialect| {
            let module = crate::lowering::lower_to_ir_with_config(
                source,
                &reg(),
                tcl_lexer::LexerConfig::for_dialect(dialect),
            );
            let Statement::Try { handlers, .. } = &module.top_level.statements[0] else {
                panic!("expected structured try");
            };
            handlers[0].trap_pattern.clone()
        };

        assert_eq!(pattern("tcl8.6"), Some(vec!["A".into(), "\u{fffd}".into()]));
        assert_eq!(pattern("tcl9.0"), Some(vec!["A".into(), "😀".into()]));
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

    // issue #1375: `while`/`for`/`foreach`/`try`/`dict for` gated their body
    // word on single-token-ness alone, not the token's kind. `$body` is a
    // single VAR token, so it passed the old gate and was lowered as if its
    // reconstructed text (`${body}`) were the literal script — miscompiling,
    // or (when the reconstruction lengthened the word, as `${body}` does
    // over `$body`) panicking in codegen on an out-of-bounds span. Each must
    // now barrier instead, mirroring `catch_dollar_var_body_falls_through_to_barrier`.

    #[test]
    fn while_dollar_var_body_falls_through_to_barrier() {
        let m = lower_to_ir("while {1} $body", &reg());
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Barrier { reason, .. } if reason == "while with dynamic arguments"
        ));
    }

    #[test]
    fn for_dollar_var_body_falls_through_to_barrier() {
        let m = lower_to_ir("for {} {1} {} $body", &reg());
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Barrier { reason, .. } if reason == "for with dynamic arguments"
        ));
    }

    #[test]
    fn foreach_dollar_var_body_falls_through_to_barrier() {
        let m = lower_to_ir("foreach x $l $body", &reg());
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Barrier { reason, .. } if reason == "foreach with dynamic body"
        ));
    }

    #[test]
    fn try_dollar_var_body_falls_through_to_barrier() {
        let m = lower_to_ir("try $body", &reg());
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Barrier { reason, .. } if reason == "try with dynamic body"
        ));
    }

    // …and the `try` bodies #1375 missed (PR #1481 review): the gate landed on
    // the primary body only, so `finally` and every non-`-` `on`/`trap`
    // handler still walked a `$body` word and rebased its reconstructed text
    // off the end of the source. tclsh 8.6.16 / 9.0.4 run both spellings fine
    // (`set body {puts hi}; try {} finally $body` prints `hi`), so the
    // degraded path must be a barrier — the runtime `try` — not a panic.
    // C's `TclCompileTryCmd` bails on the same words for the same reason.

    #[test]
    fn try_dollar_var_finally_body_falls_through_to_barrier() {
        let m = lower_to_ir("try {} finally $body", &reg());
        assert!(
            matches!(
                &m.top_level.statements[0],
                Statement::Barrier { reason, .. } if reason == "try with dynamic finally body"
            ),
            "got {:?}",
            m.top_level.statements[0],
        );
    }

    #[test]
    fn try_dollar_var_handler_body_falls_through_to_barrier() {
        for src in [
            "try {error x} on error {} $body",
            "try {error x} trap NONE {} $body",
            "try {error x} on error {} [subst $body]",
            "try {error x} on error {} $body finally {puts bye}",
        ] {
            let m = lower_to_ir(src, &reg());
            assert!(
                matches!(
                    &m.top_level.statements[0],
                    Statement::Barrier { reason, .. } if reason == "try with dynamic handler body"
                ),
                "expected a dynamic-handler-body barrier for {src:?}, got {:?}",
                m.top_level.statements[0],
            );
        }
    }

    #[test]
    fn try_static_finally_and_handler_bodies_still_lower() {
        // FP-guard: the gate narrows the walk, it does not switch it off.
        for src in [
            "try {error x} finally {puts bye}",
            "try {error x} on error {} {puts bad}",
            "try {error x} trap NONE {m o} {puts $m} finally {puts bye}",
            // The `-` fallthrough marker is not a body and keeps its own path.
            "try {set x 1} on ok a - trap NONE b {return $b}",
        ] {
            let m = lower_to_ir(src, &reg());
            assert!(
                matches!(&m.top_level.statements[0], Statement::Try { .. }),
                "expected a structured Try for {src:?}, got {:?}",
                m.top_level.statements[0],
            );
        }
    }

    #[test]
    fn dict_for_dollar_var_body_falls_through_to_barrier() {
        let m = lower_to_ir("dict for {k v} $d $body", &reg());
        assert!(matches!(
            &m.top_level.statements[0],
            Statement::Barrier { reason, .. } if reason == "dict for with dynamic body"
        ));
    }

    // issue #1431: a varList word is a Tcl list, not a whitespace split. The
    // hand-rolled splitter took only the first word of a braced element and cut
    // a backslash-escaped space in two, so both spellings of a one-variable
    // binding bound the wrong names.

    /// Loop-variable names of the single-iterator `foreach` at statement 0.
    fn foreach_vars(src: &str) -> Vec<String> {
        let m = lower_to_ir(src, &reg());
        let Statement::Foreach { iterators, .. } = &m.top_level.statements[0] else {
            panic!("expected Foreach for {src:?}, got {:?}", m.top_level);
        };
        assert_eq!(iterators.len(), 1, "one iterator for {src:?}");
        iterators[0].vars.clone()
    }

    #[test]
    fn foreach_grouped_var_element_binds_one_name() {
        // tclsh 9.0: `proc s {} { foreach {{x y}} {1} {}; info locals }`
        // reports the single local named `x y`.
        assert_eq!(foreach_vars("foreach {{x y}} {1} {puts hi}"), vec!["x y"]);
    }

    #[test]
    fn foreach_escaped_space_var_binds_one_name() {
        // Same oracle for the escaped spelling: `foreach {a\ b} {1} {}` binds
        // the one local `a b`, not the two names `a\` and `b`.
        assert_eq!(foreach_vars("foreach {a\\ b} {1} {puts hi}"), vec!["a b"]);
    }

    #[test]
    fn foreach_plain_var_list_still_binds_every_name() {
        // FP-guard: the ordinary multi-variable list is unchanged.
        assert_eq!(
            foreach_vars("foreach {m n} {1 2} {puts hi}"),
            vec!["m", "n"]
        );
        assert_eq!(foreach_vars("foreach x {1 2} {puts hi}"), vec!["x"]);
    }

    #[test]
    fn dict_for_grouped_var_element_binds_one_name() {
        assert_eq!(
            foreach_vars("dict for {{k 1} v} $d {puts hi}"),
            vec!["k 1", "v"],
        );
    }

    #[test]
    fn malformed_var_list_falls_through_to_barrier() {
        // An unbalanced varList names no binding, so each loop defers to its
        // runtime command, which raises Tcl's own list error.
        for (src, expected) in [
            (
                "foreach {a \"b} {1} {puts hi}",
                "foreach with malformed variable list",
            ),
            (
                "dict for {k \"v} $d {puts hi}",
                "dict for with malformed variable list",
            ),
            (
                "try {puts hi} on error {m \"o} {puts bad}",
                "try with malformed handler variable list",
            ),
        ] {
            let m = lower_to_ir(src, &reg());
            assert!(
                matches!(&m.top_level.statements[0], Statement::Barrier { reason, .. } if reason == expected),
                "expected {expected:?} barrier for {src:?}, got {:?}",
                m.top_level.statements[0],
            );
        }
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
