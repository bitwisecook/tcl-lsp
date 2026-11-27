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

use crate::command_binding::ExecutedScriptWords;
use std::borrow::Cow;

use tcl_lexer::{Span, Token, TokenType};

use tcl_registry::arg_role::ArgRole;
use tcl_registry::spec::CaseListSpec;
use tcl_registry::{ClauseAbstention, ClausePlan, ClauseRowId, ClauseShapeError, ClauseTiming};

use crate::expr_parser::parse_expr_for_profile_with_lexer_config;
use crate::ir::{
    ForeachIterator, HandlerMatch, IfClause, Script, Statement, SwitchArm, SwitchMode, TryHandler,
};
use crate::lowering_hooks::word_content_base;
use tcl_source::segment::SegmentedCommand;
use tcl_syntax::expr::ExprNode;

use super::Lowerer;
use tcl_syntax::word_rules::WordValueRules;

/// A single pattern/body pair collected by
/// [`lower_switch`](super::Lowerer::lower_switch) before the
/// final `SwitchArm` list is built. Factored out to keep the
/// pair vector's type signature readable.
struct SwitchPair {
    pattern: String,
    /// Whether the pattern word was braced — literal for every element of a
    /// single braced arm list, and per-word in the multi-word form.
    pattern_braced: bool,
    pattern_span: Span,
    body_text: String,
    body_span: Option<Span>,
    body_arg_idx: Option<usize>,
    body_source: Option<tcl_names::origin::ExecutedScriptSource>,
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
/// ordinary pattern character.  A script lexer that skips
/// `TokenType::Comment` tokens silently deletes a valid `#` pattern and its
/// body (tclsh 9.0.4: `switch # { # {puts matched} default {puts default} }`
/// prints `matched`).
///
/// A bare or quoted element's backslashes collapse under the document's escape
/// grammar (`escapes`): before Tcl 8.6 a `\x` takes every hex digit that
/// follows and keeps the last two.
///
/// Returns `None` when the text is not a well-formed list — the caller
/// bails the whole `switch` to the runtime command, which reports the
/// error exactly as C Tcl does.
fn switch_body_elements(
    body_text: &str,
    escapes: tcl_lexer::EscapeSyntax,
) -> Option<Vec<SwitchElement>> {
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
                    tcl_lexer::backslash_subst_in(&raw, escapes).into_owned()
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

/// Whether the `index`-th argument was written as a braced word.
///
/// One predicate, because two callers ask it: the `switch` subject's own flag
/// and the per-argument flags the generic fallback pushes by. `TokenType::Str`
/// is the braced word, and the single-token check excludes a composite word
/// that merely starts with a brace — `arg_single` alone is *not* the question,
/// since `$x` and a bare word satisfy it too.
fn word_is_braced(arg_tokens: &[tcl_lexer::Token], arg_single: &[bool], index: usize) -> bool {
    matches!(
        arg_tokens.get(index).map(|t| t.kind),
        Some(tcl_lexer::TokenType::Str)
    ) && arg_single.get(index).copied().unwrap_or(false)
}

/// [`word_is_braced`] for every argument, for the generic-invoke fallback.
pub(crate) fn braced_word_flags(
    arg_tokens: &[tcl_lexer::Token],
    arg_single: &[bool],
    len: usize,
) -> Vec<bool> {
    (0..len)
        .map(|i| word_is_braced(arg_tokens, arg_single, i))
        .collect()
}

/// Per argument, whether the word was double-quoted: its representative
/// token opens at the `"` (a quoted-opening `Esc` token counts the quote as
/// a delimiter byte) or runs inside the quotes.
pub(crate) fn quoted_word_flags(arg_tokens: &[tcl_lexer::Token], len: usize) -> Vec<bool> {
    (0..len)
        .map(|i| {
            arg_tokens.get(i).is_some_and(|token| {
                token.in_quote || (token.kind == TokenType::Esc && token.content_offset > 0)
            })
        })
        .collect()
}

/// The IR's mode for a case list's own comparison, or `None` for a
/// specialised one the statement cannot carry.
const fn switch_mode_of(mode: tcl_registry::spec::CaseMatchMode) -> Option<SwitchMode> {
    match mode {
        tcl_registry::spec::CaseMatchMode::Exact => Some(SwitchMode::Exact),
        tcl_registry::spec::CaseMatchMode::Glob => Some(SwitchMode::Glob),
        tcl_registry::spec::CaseMatchMode::Regexp => Some(SwitchMode::Regexp),
        tcl_registry::spec::CaseMatchMode::Other => None,
    }
}

/// Where a case list's options end and how its clauses compare, as
/// `(subject_index, mode, nocase)`; `Err` is the reason the whole command
/// defers to the runtime. A command with option rows scans them
/// ([`parse_switch_options`]); one with none (`case`) scans no word as an
/// option — its subject is the first word — and every clause compares as
/// its descriptor states, `Tcl_CaseObjCmd`'s glob for `case`.
fn case_list_head(
    case: &CaseListSpec,
    reads_options: bool,
    args: &[String],
) -> Result<(usize, SwitchMode, bool), &'static str> {
    if !reads_options {
        let mode =
            switch_mode_of(case.default_mode).ok_or("case list with a specialised comparison")?;
        return Ok((0, mode, false));
    }
    let (i, mode, nocase, unknown, _) = parse_switch_options(args);
    // An unrecognised / arg-taking option (`-foo`, `-matchvar`, …): bail to
    // the runtime `switch`, which validates options and does the var writes.
    if unknown {
        return Err("switch with non-inlined option");
    }
    Ok((i, mode, nocase))
}

/// Why the IR's arm cannot hold a clause of a case list, when one cannot:
/// under a descriptor whose pattern words may be lists (`case`), a pattern
/// the command splits as one ([`CaseListSpec::pattern_is_list`]) or whose
/// value the source does not state, which may be one; and, under one whose
/// keyword pattern counts wherever it stands, a keyword before the last
/// clause — the IR keeps the fallback as its default body, which runs only
/// after every arm.
fn case_list_unrepresentable(case: &CaseListSpec, pairs: &[SwitchPair]) -> Option<&'static str> {
    let is_list = |pair: &SwitchPair| {
        case.pattern_is_list(&pair.pattern)
            || (case.pattern_words == tcl_registry::spec::PatternWords::Lists
                && !pair.pattern_braced
                && (pair.pattern.contains('$') || pair.pattern.contains('[')))
    };
    if pairs.iter().any(is_list) {
        return Some("case pattern list");
    }
    let early_keyword = pairs.split_last().is_some_and(|(_, rest)| {
        rest.iter()
            .any(|pair| case.keyword_patterns.contains(&pair.pattern.as_str()))
    });
    if !case.keyword_patterns_require_final && early_keyword {
        return Some("case keyword pattern before the last clause");
    }
    None
}

/// Parse switch options, returning `(first_non_option_index, mode, nocase,
/// unknown, ended)`. `unknown` is set when a leading `-word` is not one of the
/// options the compiler inlines (`-exact`/`-glob`/`-regexp`/`-nocase`/`--`) —
/// an arg-taking `-indexvar`/`-matchvar`, or an invalid option such as `-foo`.
/// The caller bails the whole switch to the runtime command, which validates
/// the option set (tclsh rejects `-foo`) and handles the side-channel writes.
/// `ended` is set when `--` closed the options, so the next word is the
/// subject whatever it spells.
///
/// `pub(crate)` so the dispatch chain's lowering can ask the same question
/// this answers for this lowering — which argument the subject is, and whether
/// `--` stands before it — rather than keeping a second copy of the option
/// rule. There is one owner of "where do the options end", and this is it.
pub(crate) fn parse_switch_options(args: &[String]) -> (usize, SwitchMode, bool, bool, bool) {
    let mut i = 0;
    let mut mode = SwitchMode::Exact;
    let mut nocase = false;
    let mut unknown = false;
    let mut ended = false;
    while i < args.len() && args[i].starts_with('-') {
        match args[i].as_str() {
            // registry-axis-ok: options — switch's match-mode options; until the switch option table selects the match mode
            "--" => {
                i += 1;
                ended = true;
                break;
            }
            // registry-axis-ok: options — switch's match-mode options; until the switch option table selects the match mode
            "-exact" => mode = SwitchMode::Exact,
            // registry-axis-ok: options — switch's match-mode options; until the switch option table selects the match mode
            "-glob" => mode = SwitchMode::Glob,
            // registry-axis-ok: options — switch's match-mode options; until the switch option table selects the match mode
            "-regexp" => mode = SwitchMode::Regexp,
            // registry-axis-ok: options — switch's match-mode options; until the switch option table selects the match mode
            "-nocase" => nocase = true,
            _ => {
                unknown = true;
                break;
            }
        }
        i += 1;
    }
    (i, mode, nocase, unknown, ended)
}

/// The pattern/body pairs of `switch`'s multi-word form, from word `start`
/// on; `Err` is the reason the whole `switch` defers to the runtime command.
fn inline_switch_pairs(
    seg: &SegmentedCommand,
    case: &CaseListSpec,
    start: usize,
) -> Result<Vec<SwitchPair>, &'static str> {
    let args = seg.args();
    let arg_tokens = seg.arg_tokens();
    let arg_single = seg.arg_single_token();
    if !(args.len() - start).is_multiple_of(2) {
        return Err("switch odd pattern count");
    }
    let mut pairs = Vec::with_capacity((args.len() - start) / 2);
    for i in (start..args.len()).step_by(2) {
        let body_tok_idx = i + 1;
        // Like if/while/for/catch/try, an arm body that carries substitution
        // (`$handler`, `[cmd]`, a quoted/concatenated word) is evaluated as a
        // script from its *runtime value*, not its unsubstituted spelling.
        // Lowering the literal spelling as a nested script would fabricate a
        // phantom command (e.g. a Call to `${handler}`) that downstream
        // dead-code/taint/def-use/call-graph passes reason about. Defer the
        // whole switch to the runtime command instead. The case list's
        // fall-through body is a literal, not a body, so it is exempt.
        // (`seg.argv` includes the command word, so the body word
        // `args[i + 1]` is index `i + 2`.)
        if case.fallthrough_body != Some(args[body_tok_idx].as_str())
            && !super::seg_word_is_static_literal(seg, i + 2)
        {
            return Err("switch with non-literal arm body");
        }
        pairs.push(SwitchPair {
            pattern: args[i].clone(),
            // Per word here: `{${x}}` is literal, a bare `$pat` substitutes.
            pattern_braced: word_is_braced(arg_tokens, arg_single, i),
            pattern_span: arg_tokens.get(i).map_or(seg.span, |t| t.span),
            body_text: args[body_tok_idx].clone(),
            body_span: arg_tokens.get(body_tok_idx).map(|t| t.span),
            body_arg_idx: Some(body_tok_idx),
            body_source: None,
        });
    }
    Ok(pairs)
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

/// The barrier reason for an `if` whose clause plan has `defect`: the
/// runtime `if`'s own error, in the words the lowering has always used.
fn if_defect_reason(plan: &ClausePlan, defect: ClauseShapeError) -> &'static str {
    let explicit_else = plan
        .clauses
        .last()
        .is_some_and(|clause| clause.row == ClauseRowId::Tail && clause.keyword_index.is_some());
    match defect {
        ClauseShapeError::MissingExpr { after: None } => "malformed if",
        ClauseShapeError::MissingExpr { after: Some(_) } => "if missing elseif expression",
        ClauseShapeError::MissingBody { .. } if explicit_else => "malformed if else clause",
        ClauseShapeError::MissingBody { .. } => "malformed if clause",
        ClauseShapeError::ExtraWords { .. } if explicit_else => "if extra words after else",
        ClauseShapeError::ExtraWords { .. } => "if with extra words",
    }
}

impl Lowerer<'_> {
    /// The condition of `if`/`while`/`for` from argv word `word`. A simple
    /// word is the expression itself; any other word is substituted and its
    /// value evaluated as an expression (`TclCompileExprWords`), so `if $c`
    /// with `c` holding `1==1` is true ([`ExprNode::substituted_word`]).
    fn condition_expr(&self, seg: &SegmentedCommand, word: usize, text: &str) -> ExprNode {
        if super::seg_word_is_simple(seg, word) {
            return parse_expr_for_profile_with_lexer_config(text, self.dialect, self.config);
        }
        ExprNode::substituted_word(text)
    }

    // if

    /// The clause walk of `seg` under this lowerer's registry and release:
    /// the command's clause grammar over the words' *values* — a substituted
    /// word is computed, a backslash-escaped one decoded — so every keyword,
    /// noise word and fall-through marker is compared as Tcl compares it
    /// (`\-` is `try`'s marker, `{\-}` is not). `None` when the head resolves
    /// no grammar at this release; `Err` where a computed word stands where
    /// the walk compares one.
    fn clause_walk_of(
        &self,
        seg: &SegmentedCommand,
    ) -> Option<Result<ClausePlan, ClauseAbstention>> {
        let tokens = self.cmd_tokens(seg);
        let context = self.invocation_metadata_context();
        let invocation = self.logical_structured_invocation(&tokens)?;
        let realm = tokens
            .source_binding
            .as_ref()
            .and_then(crate::command_binding::SourceInvocationBinding::invocation_realm)
            .unwrap_or(self.invocation_realm);
        invocation.with_lowering_source_schema(self.registry, context, realm, |selected| {
            selected.clause_walk()
        })
    }

    /// Lower `if cond ?then? body ?elseif cond ?then? body ...? ?else? ?body?`
    /// from the command's clause plan: each clause with a condition becomes
    /// an [`IfClause`], the default clause the `else` body. Keywords and the
    /// `then` noise word are the plan's; nothing here compares a spelling.
    pub(super) fn lower_if(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let Some(walk) = self.clause_walk_of(seg) else {
            return self.barrier(seg, "malformed if");
        };
        // A computed word where a keyword could stand (`if $c {a} $w {b}`)
        // leaves the chain to the runtime `if`; the inert reading still says
        // how far the literal clauses go before it defers.
        let (plan, computed) = match walk {
            Ok(plan) => (plan, false),
            Err(abstention) => (abstention.inert, true),
        };
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();

        let mut clauses = Vec::new();
        let mut else_body = None;
        let mut else_span = None;
        // Reachability tracking. Once a clause's condition
        // folds to a static `true`, every later clause + the
        // ``else`` branch is dead. A clause whose own condition
        // is a static `false` is dead this iteration but not
        // necessarily later — track per-clause via the
        // ``dead_code_depth`` counter.
        let mut later_clauses_dead = false;

        let last = plan.clauses.len().saturating_sub(1);
        for (index, clause) in plan.clauses.iter().enumerate() {
            // With a defect, the last clause is the one the defect stopped in
            // or the one extra words follow (`if 0 {a} else {b} junk`): the
            // runtime `if` reports it faithfully (if-2.3, if-3.5), so the
            // construct defers to it before that clause is lowered.
            if plan.defect.is_some() && index == last {
                break;
            }
            let Some(body_idx) = clause.operand(ArgRole::Body) else {
                return self.barrier(seg, "malformed if clause");
            };
            // C's TclCompileIfCmd only inlines a braced-literal body; a body
            // carrying substitutions (`$x`, `[cmd]`, a quoted or concatenated
            // word like `$x1$x2`) must be substituted *then* evaluated as a
            // script — which the runtime `if` command does. Bail the whole
            // construct to that command rather than mis-parsing the unsubstituted
            // word as a literal script at compile time.
            if !super::seg_word_is_simple(seg, body_idx + 1) {
                return self.barrier(seg, "if with non-literal body");
            }
            let body_tok = arg_tokens.get(body_idx);
            let Some(cond_idx) = clause.operand(ArgRole::Expr) else {
                // Only the default clause — `else`, or the bare final body its
                // optional keyword allows — runs without a condition.
                if !clause.is_default {
                    return self.barrier(seg, "malformed if clause");
                }
                let dead = later_clauses_dead;
                if dead {
                    self.dead_code_depth += 1;
                }
                else_body = Some(self.lower_body_from_tok(&args[body_idx], body_tok, namespace));
                if dead {
                    self.dead_code_depth -= 1;
                }
                else_span = body_tok.map(|t| self.script_word_span(t));
                continue;
            };
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
                condition: self.condition_expr(seg, cond_idx + 1, &cond_text),
                condition_span: cond_tok.map_or(seg.span, |t| t.span),
                condition_base: cond_tok
                    .and_then(|t| word_content_base(t.span, cond_single, &cond_text)),
                body,
                body_span: body_tok.map_or(seg.span, |t| self.script_word_span(t)),
            });
            // Static-true condition latches the dead-code flag so
            // remaining clauses + the else branch are suppressed.
            if matches!(static_cond, Some(true)) {
                later_clauses_dead = true;
            }
        }

        if let Some(defect) = plan.defect {
            return self.barrier(seg, if_defect_reason(&plan, defect));
        }
        if computed {
            return self.barrier(seg, "if with a computed clause word");
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
        // lowered as if it were the literal text.
        if !arg_single[1]
            || !super::seg_word_is_simple(seg, 1)
            || !super::seg_word_is_simple(seg, 3)
            || !super::seg_word_is_simple(seg, 4)
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
            init_span: self.script_word_span(&arg_tokens[0]),
            condition: self.condition_expr(
                seg,
                2,
                &condition_source_text(arg_tokens.get(1), arg_single[1], &args[1]),
            ),
            condition_span: arg_tokens[1].span,
            condition_base: word_content_base(
                arg_tokens[1].span,
                arg_single[1],
                &condition_source_text(arg_tokens.get(1), arg_single[1], &args[1]),
            ),
            next,
            next_span: self.script_word_span(&arg_tokens[2]),
            body,
            body_span: self.script_word_span(&arg_tokens[3]),
            raw_args: args.to_vec(),
            raw_tokens: Some(Box::new(self.cmd_tokens(seg))),
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
        // as if it were the literal text.
        if !arg_single[0] || !super::seg_word_is_simple(seg, 2) {
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
            condition: self.condition_expr(
                seg,
                1,
                &condition_source_text(arg_tokens.first(), arg_single[0], &args[0]),
            ),
            condition_span: arg_tokens[0].span,
            condition_base: word_content_base(
                arg_tokens[0].span,
                arg_single[0],
                &condition_source_text(arg_tokens.first(), arg_single[0], &args[0]),
            ),
            body,
            body_span: self.script_word_span(&arg_tokens[1]),
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
        // as if it were the literal text.
        if body_tok.is_none() || !super::seg_word_is_static_literal(seg, body_idx + 1) {
            return self.barrier(seg, "foreach with dynamic body");
        }

        // The value word's brace-quoting is a fact only the tokens carry:
        // `foreach n {a $b c}` iterates the three literal elements `a`,
        // `$b`, `c` and reads nothing (tclsh 8.6.14), while `foreach n
        // "a $b c"` substitutes. Both lower to the same `list_arg` text,
        // so the flag is what downstream read-harvesting gates on.
        let cmd_tokens = self.cmd_tokens(seg);
        let mut iterators = Vec::new();
        for i in (0..body_idx).step_by(2) {
            // A varList that is not a well-formed Tcl list binds nothing we can
            // name; the runtime `foreach` raises Tcl's own error.
            let Some(var_names) = crate::registry_invocation::original_name_role_values(
                self.registry,
                &cmd_tokens,
                i,
                ArgRole::LoopVarList,
            ) else {
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
        let lmap_needs_runtime = is_lmap && !body.is_straight_line();
        if self.target.is_bytecode() && (lmap_needs_runtime || body_nests_foreach) {
            return self.barrier(seg, if is_lmap { "lmap" } else { "foreach" });
        }

        Statement::Foreach {
            span: seg.span,
            iterators,
            body,
            body_span: body_tok.map_or(seg.span, |t| self.script_word_span(t)),
            is_lmap,
            raw_args: args.to_vec(),
            is_dict_iteration: false,
            is_array_iteration: false,
            raw_tokens: Some(cmd_tokens),
        }
    }

    /// Bind one decoded name for file-line analysis. Execution consumers must
    /// retain the selected file-iteration descriptor rather than list semantics.
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
        if !self.can_lower_body_word(seg, 3) {
            return self.barrier(seg, "foreachLine with dynamic body");
        }

        // The selected scalar receiver owns one name; the iteration source
        // remains a filename and supplies no list-iteration execution contract.
        let cmd_tokens = self.cmd_tokens(seg);
        let Some(vars) = crate::registry_invocation::original_name_role_values(
            self.registry,
            &cmd_tokens,
            0,
            ArgRole::VarWrite,
        ) else {
            return self.barrier(seg, "foreachLine with dynamic variable name");
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
            body_span: body_tok.map_or(seg.span, |t| self.script_word_span(t)),
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

        let arity = self
            .invocation_dialect
            .and_then(tcl_registry::catch_invocation::catch_positional_arity);
        if arity
            .is_none_or(|arity| !u16::try_from(args.len()).is_ok_and(|count| arity.accepts(count)))
        {
            return self.barrier(seg, "malformed catch");
        }
        // Body must be a single brace-literal (`Str` kind) token to
        // compile statically.  Variable references (`$cmd`) and
        // bracket commands (`[expr ...]`) are single-token but
        // non-`Str` and must fall through to the runtime
        // `eval_catch`, which calls `eval_script` on the substituted
        // value.  Without the kind check, ``catch $cmd res`` would
        // be compiled as "call the proc named by ``$cmd``" — wrong.
        if arg_tokens.is_empty() || !self.can_lower_body_word(seg, 1) {
            return self.barrier(seg, "catch with dynamic body");
        }

        let body = self.lower_body_from_tok(&args[0], Some(&arg_tokens[0]), namespace);
        let receiver = |argument| self.original_control_receiver_value(seg, argument);
        let result_var = if args.len() > 1 {
            let Some(name) = receiver(1) else {
                return self.barrier(seg, "catch with dynamic receiver");
            };
            Some(name)
        } else {
            None
        };
        let options_var = if args.len() > 2 {
            let Some(name) = receiver(2) else {
                return self.barrier(seg, "catch with dynamic receiver");
            };
            Some(name)
        } else {
            None
        };

        Statement::Catch {
            span: seg.span,
            body,
            body_span: self.script_word_span(&arg_tokens[0]),
            result_var,
            options_var,
            raw_args: args.to_vec(),
            tokens: Some(self.cmd_tokens(seg)),
        }
    }

    /// Receiver operands are evaluated name data, never variable-reference syntax.
    fn original_control_receiver_value(
        &self,
        seg: &SegmentedCommand,
        argument: usize,
    ) -> Option<String> {
        if self.target.is_bytecode() {
            return self.original_native_written_argument_value(seg, argument);
        }
        self.logical_structured_invocation(&self.cmd_tokens(seg))?
            .argument_literal(argument)
    }

    // try

    /// Lower `try body ?handler ...? ?finally finallyBody?` from the command's
    /// clause plan: the protected clause is the body, each clause its
    /// pattern selects becomes a [`TryHandler`] carrying the row's
    /// [`HandlerMatch`], and the clause that runs whatever the outcome is
    /// `finally`. Keywords and the `-` marker are the plan's.
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

        if args.is_empty() {
            return self.barrier(seg, "malformed try");
        }
        let Some(walk) = self.clause_walk_of(seg) else {
            return self.barrier(seg, "malformed try");
        };
        // A computed word where a keyword or the marker could stand leaves
        // the chain to the runtime `try`; the inert reading still says how far
        // the literal clauses go before it defers.
        let (plan, computed) = match walk {
            Ok(plan) => (plan, false),
            Err(abstention) => (abstention.inert, true),
        };
        let Some((protected, rest)) = plan.clauses.split_first() else {
            return self.barrier(seg, "malformed try");
        };
        let Some(body_idx) = protected.operand(ArgRole::Body) else {
            return self.barrier(seg, "malformed try");
        };
        // The body is lowered via `lower_body_from_tok`, which rebases the
        // segmenter-reconstructed word text at the token's span — safe only
        // for a brace-literal (`Str`) token, matching `lower_catch`'s body
        // guard. A single-token but dynamic body (`$body`, `[cmd]`) must
        // barrier rather than be lowered as if it were the literal text.
        let Some(body_tok) = arg_tokens.get(body_idx) else {
            return self.barrier(seg, "try with dynamic body");
        };
        if !self.can_lower_body_word(seg, body_idx + 1) {
            return self.barrier(seg, "try with dynamic body");
        }

        let body = self.lower_body_from_tok(&args[body_idx], Some(body_tok), namespace);
        let mut handlers = Vec::new();
        let mut finally_body = None;
        let mut finally_span = None;

        for (offset, clause) in rest.iter().enumerate() {
            // A clause the defect stopped in has no script word of its own.
            let Some(clause_body) = clause.operand(ArgRole::Body) else {
                break;
            };
            let clause_tok = arg_tokens.get(clause_body);
            match clause.timing {
                ClauseTiming::Always => {
                    // The `finally` word is a body like any other, so it needs
                    // the same static gate as the primary body above:
                    // single-token-ness alone would let `try {} finally $body`
                    // through, and rebasing the reconstructed `${body}` text at
                    // the `$body` token's span puts the inner statement off the
                    // end of the source. C's `TclCompileTryCmd` makes the same
                    // call — a `finally` word that is not a
                    // `TCL_TOKEN_SIMPLE_WORD` is `goto failedToCompile`,
                    // deferring the whole `try` to the runtime command.
                    if clause_tok.is_none() || !self.can_lower_body_word(seg, clause_body + 1) {
                        return self.barrier(seg, "try with dynamic finally body");
                    }
                    finally_body =
                        Some(self.lower_body_from_tok(&args[clause_body], clause_tok, namespace));
                    finally_span = clause_tok.map(|t| t.span);
                }
                ClauseTiming::Selected => {
                    let (Some((pattern_idx, kind)), Some(var_list_idx)) =
                        (clause.handler(), clause.operand(ArgRole::LoopVarList))
                    else {
                        return self.barrier(seg, "malformed try handler");
                    };
                    match self.lower_try_handler(
                        seg,
                        namespace,
                        kind,
                        [pattern_idx, var_list_idx, clause_body],
                        plan.falls_through(offset + 1),
                    ) {
                        Ok(handler) => handlers.push(handler),
                        Err(reason) => return self.barrier(seg, reason),
                    }
                }
                _ => return self.barrier(seg, "malformed try handler"),
            }
        }

        if plan.defect.is_some() || computed {
            return self.barrier(seg, "malformed try handler");
        }

        Statement::Try {
            span: seg.span,
            body,
            body_span: self.script_word_span(body_tok),
            handlers,
            finally_body,
            finally_span,
            raw_args: args.to_vec(),
        }
    }

    /// Decode only a statically selected trap operand through the retained grammar.
    fn try_trap_pattern(
        &self,
        seg: &SegmentedCommand,
        written: usize,
        keyword: &str,
        match_arg: &str,
    ) -> Result<Option<Vec<String>>, ()> {
        // registry-axis-ok: clause_grammar — try's trap clause; until the clause plan names the trap handler
        if keyword != "trap" || !super::seg_word_is_static_literal(seg, written) {
            return Ok(None);
        }
        let match_tok = seg.arg_tokens().get(written - 1);
        let value = if match_tok.is_some_and(|tok| tok.kind == TokenType::Str) {
            std::borrow::Cow::Borrowed(match_arg)
        } else {
            let bytes =
                tcl_lexer::backslash_subst_bytes_in(match_arg.as_bytes(), self.config.escapes);
            let text = std::str::from_utf8(&bytes).map_err(|_| ())?;
            std::borrow::Cow::Owned(text.to_owned())
        };
        Ok(WordValueRules::from_config(&self.config)
            .split_list(&value)
            .ok()
            .map(|elements| elements.into_iter().map(Into::into).collect()))
    }

    /// One handler clause of [`Self::lower_try`]: `words` are its pattern,
    /// variable-list and script word indices, and `fallthrough` whether the
    /// plan reads the script word as the grammar's fall-through marker. `Err`
    /// is the reason the whole `try` defers to the runtime command.
    fn lower_try_handler(
        &mut self,
        seg: &SegmentedCommand,
        namespace: &str,
        kind: HandlerMatch,
        [pattern_idx, var_list_idx, body_idx]: [usize; 3],
        fallthrough: bool,
    ) -> Result<TryHandler, &'static str> {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let match_arg = args[pattern_idx].clone();
        // An error-code-prefix selector (`trap`'s) is evaluated as one Tcl
        // word and its value is then parsed as a Tcl list. Preserve the
        // source-level substitution decision here: inspecting the decoded
        // elements for `$` or `[` would reject literal data such as
        // `{A {$B}}` and `{A \$B}`. A substitution-free bare/quoted word still
        // needs backslash substitution before list parsing, whereas a braced
        // word's content is already its literal runtime value.
        let keyword = if kind == HandlerMatch::ErrorCodePrefix {
            "trap"
        } else {
            "on"
        };
        let trap_pattern = self
            .try_trap_pattern(seg, pattern_idx + 1, keyword, &match_arg)
            .map_err(|()| "try trap native byte pattern is not represented")?;
        let Some(variable_list) = self.original_control_receiver_value(seg, var_list_idx) else {
            return Err("try with dynamic handler variable list");
        };
        let Ok(var_names) = WordValueRules::from_config(&self.config).split_list(&variable_list)
        else {
            return Err("try with malformed handler variable list");
        };
        let var_name = var_names.first().map(ToString::to_string);
        let options_var = var_names.get(1).map(ToString::to_string);
        let handler_tok = arg_tokens.get(body_idx);
        // A handler whose script word is the fall-through marker shares the
        // next handler's body (like `switch`). Tcl recognises the marker by
        // the word's *value*, and so does the plan: the braced `{-}`, quoted
        // `"-"` and backslash-escaped (`\-`, `\x2d`) spellings all evaluate to
        // `-`, while braces suppress backslash substitution, so `{\-}` is the
        // two-character script `\-`. The marker is lowered as an empty script
        // — as a script it would be a zero-arg call of the `-` command and
        // trip a spurious arity error.
        //
        // Every other handler script gets the primary body's static gate:
        // `on error {} $body` is a single VAR token, so an unconditional walk
        // would rebase the reconstructed `${body}` text at that token and
        // emit a span past the end of the source. C's `TclCompileTryCmd`
        // refuses the same shape — a handler body that is not a
        // `TCL_TOKEN_SIMPLE_WORD` is `goto failedToCompile` — so the whole
        // `try` defers to the runtime command, exactly as the primary-body
        // gate does.
        if !fallthrough && (handler_tok.is_none() || !self.can_lower_body_word(seg, body_idx + 1)) {
            return Err("try with dynamic handler body");
        }
        let body = if fallthrough {
            crate::ir::Script::new()
        } else {
            self.lower_body_from_tok(&args[body_idx], handler_tok, namespace)
        };
        Ok(TryHandler {
            kind,
            match_arg,
            trap_pattern,
            var_name,
            options_var,
            body,
            body_span: handler_tok.map_or(seg.span, |t| self.script_word_span(t)),
            fallthrough,
        })
    }

    // switch

    /// Lower `switch ?options? subject pattern body ...`.
    // Sequential `switch` lowering: option parsing, list-form
    // unpacking, body recursion, and case-list build all share
    // local arena state.
    /// The case list `seg`'s command declares — its fall-through body and its
    /// keyword patterns are the registry's, never spellings in the lowering.
    /// The call's case-list descriptor, and whether its command reads any
    /// option rows — `case` has none, so no word of it is an option.
    fn case_list_of(&self, seg: &SegmentedCommand) -> Option<(CaseListSpec, bool)> {
        let tokens = self.cmd_tokens(seg);
        let context = self.invocation_metadata_context();
        let invocation = self.logical_structured_invocation(&tokens)?;
        let realm = tokens
            .source_binding
            .as_ref()
            .and_then(crate::command_binding::SourceInvocationBinding::invocation_realm)
            .unwrap_or(self.invocation_realm);
        invocation.with_lowering_source_schema(self.registry, context, realm, |selected| {
            Some((
                *selected.semantics.options.case_list?,
                !selected.semantics.options.base.is_empty()
                    || !selected.semantics.options.form.is_empty(),
            ))
        })
    }

    /// Build the `(arms, default_body, default_span)` triple from
    /// collected `SwitchPair` entries.  Extracted from
    /// [`Self::lower_switch`] to keep the dispatcher under threshold.
    fn build_switch_arms(
        &mut self,
        case: &CaseListSpec,
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
            // The case list's fall-through body shares the next arm's.
            if case.fallthrough_body == Some(pair.body_text.as_str()) {
                arms.push(SwitchArm {
                    pattern,
                    pattern_braced: pair.pattern_braced,
                    pattern_span: pair.pattern_span,
                    body: None,
                    body_span: None,
                    fallthrough: true,
                });
                continue;
            }

            let body_tok = pair.body_arg_idx.and_then(|idx| arg_tokens.get(idx));
            let body = if let Some(source) = &pair.body_source {
                self.lower_retained_script(source.clone())
            } else if let Some(tok) = body_tok {
                self.lower_body_from_tok(&pair.body_text, Some(tok), namespace)
            } else {
                Script::new()
            };

            // A keyword pattern (`default`) matches unconditionally — in the
            // final clause only, where the case list says so.
            let final_clause = pair_idx == pairs.len() - 1;
            if case.keyword_patterns.contains(&pattern.as_str())
                && (final_clause || !case.keyword_patterns_require_final)
            {
                default_body = Some(body);
                default_span = pair.body_span;
            } else {
                arms.push(SwitchArm {
                    pattern,
                    pattern_braced: pair.pattern_braced,
                    pattern_span: pair.pattern_span,
                    body: Some(body),
                    body_span: pair.body_span,
                    fallthrough: false,
                });
            }
        }
        (arms, default_body, default_span)
    }

    fn switch_list_source(
        &self,
        seg: &SegmentedCommand,
        argument: usize,
    ) -> Option<tcl_names::origin::ExecutedScriptSource> {
        let bindings = self.source_bindings.as_ref()?;
        let origin = bindings.source_origin()?.clone();
        let tokens = self.cmd_tokens(seg);
        let word = tokens.words().get(argument + 1)?;
        let crate::registry_invocation::EffectiveInvocationWord::Literal(value) =
            crate::registry_invocation::effective_invocation_word(
                word,
                self.config.escapes,
                WordValueRules::from_config(&self.config),
            )
        else {
            return None;
        };
        Some(tcl_names::origin::ExecutedScriptSource::from_word(
            tcl_names::command::CommandAllocationSite {
                source: origin,
                offset: seg.span.start(),
            },
            argument,
            word,
            &value,
            self.config,
        ))
    }

    fn switch_list_pairs(
        &self,
        seg: &SegmentedCommand,
        i: usize,
    ) -> Result<Vec<SwitchPair>, &'static str> {
        let arg_tokens = seg.arg_tokens();
        let mut pairs = Vec::new();
        let Some(list_source) = self.switch_list_source(seg, i) else {
            return Err("switch case list value is unproved");
        };
        let body_text = list_source
            .try_text()
            .map_err(|_| "switch case list has no Unicode analysis view")?;
        // Starting offset of the body *content* inside the
        // outer source. For a braced word the content begins
        // one byte after the opening `{`.
        let body_base = list_source.base();

        // Not a well-formed Tcl list — bail to the runtime `switch`,
        // which reports the list error exactly as C Tcl does.
        let Some(elements) = switch_body_elements(body_text, self.config.escapes) else {
            return Err("switch case list is not a list");
        };
        // An empty arm list (`switch x {}`) is a "wrong # args" error, not a
        // no-op — bail to the runtime command, which reports it.
        if elements.is_empty() {
            return Err("switch with no arms");
        }
        if !elements.len().is_multiple_of(2) {
            return Err("switch odd pattern count");
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
                // Every element of a single braced arm list is literal.
                pattern_braced: true,
                pattern_span: relocate(pat.span),
                body_text: body.value.clone(),
                body_span: Some(relocate(body.span)),
                body_arg_idx: None,
                body_source: self
                    .source_bindings
                    .as_ref()
                    .and_then(|bindings| {
                        bindings.executed_script_for_list_element(arg_tokens[i].span, j + 1)
                    })
                    .cloned()
                    .or_else(|| {
                        list_source.list_element(
                            tcl_names::command::CommandAllocationSite {
                                source: self
                                    .source_bindings
                                    .as_ref()
                                    .and_then(|bindings| bindings.source_origin())
                                    .cloned()
                                    .unwrap_or_else(|| list_source.origin.clone()),
                                offset: seg.span.start(),
                            },
                            i,
                            j + 1,
                            WordValueRules::from_config(&self.config),
                        )
                    }),
            });
            j += 2;
        }
        Ok(pairs)
    }

    pub(super) fn lower_switch(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();

        if args.len() < 2 {
            return self.lower_default(seg, namespace);
        }
        let Some((case, reads_options)) = self.case_list_of(seg) else {
            return self.barrier(seg, "switch without a case list");
        };

        let (mut i, mode, nocase) = match case_list_head(&case, reads_options, args) {
            Ok(head) => head,
            Err(reason) => return self.barrier(seg, reason),
        };
        if i >= args.len() {
            return self.lower_default(seg, namespace);
        }

        let subject = args[i].clone();
        // Whether the subject word was *braced*, which decides whether its
        // value is literal. `TokenType::Str` is the braced word plus a
        // single-token check, which is the predicate the rest of lowering uses
        // for this question (`braced_at` in `lowering::mod`). `arg_single`
        // alone is not it — it means "one token", which `$x` and a bare word
        // also satisfy, and bracing those would freeze a subject that must
        // substitute.
        let subject_braced = word_is_braced(arg_tokens, arg_single, i);
        i += 1;
        // The descriptor's literal separator (`case string ?in? …`), skipped
        // when the word's value is it, as `Tcl_CaseObjCmd`'s `strcmp` does. A
        // word there whose value the source does not state could be it.
        if let Some(separator) = case.optional_subject_separator
            && i < args.len()
        {
            if !super::seg_word_is_static_literal(seg, i + 1) || args[i].contains('\\') {
                return self.barrier(seg, "case list separator is computed");
            }
            if args[i] == separator {
                i += 1;
            }
        }
        if i >= args.len() {
            return self.lower_default(seg, namespace);
        }

        // Collect pattern/body pairs. Each pair carries a
        // `body_span` resolved in the outer source's offset
        // space plus an optional `body_arg_idx` into
        // `arg_tokens` so the body lowerer can still pick up
        // the right content offset / encoding flags.
        // A single braced list supplies literal patterns; separate words
        // retain their runtime substitutions.
        let (pairs, patterns_braced) =
            if i == args.len() - 1 && i < arg_single.len() && arg_single[i] {
                match self.switch_list_pairs(seg, i) {
                    Ok(pairs) => (pairs, true),
                    Err(_) => return self.lower_default(seg, namespace),
                }
            } else {
                match inline_switch_pairs(seg, &case, i) {
                    Ok(mut pairs) => {
                        // A body word's span runs through its closing
                        // delimiter, as every other body span does.
                        for pair in &mut pairs {
                            pair.body_span = pair
                                .body_span
                                .map(|span| tcl_lexer::word_span_at(&self.source, span));
                        }
                        (pairs, false)
                    }
                    Err(reason) => return self.barrier(seg, reason),
                }
            };

        if let Some(reason) = case_list_unrepresentable(&case, &pairs) {
            return self.barrier(seg, reason);
        }
        let (arms, default_body, default_span) =
            self.build_switch_arms(&case, &pairs, arg_tokens, namespace);

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
            raw_arg_braced: braced_word_flags(arg_tokens, arg_single, args.len()),
            raw_arg_quoted: quoted_word_flags(arg_tokens, args.len()),
            command: seg.name().to_owned(),
            patterns_braced,
        }
    }

    // dict

    fn original_dict_write_name(&self, seg: &SegmentedCommand) -> Option<(String, bool)> {
        let tokens = self.cmd_tokens(seg);
        let context = self.invocation_metadata_context()?;
        let invocation = if self.target.is_bytecode() {
            self.resolved_metadata_invocation(&tokens)?
        } else {
            crate::registry_invocation::original_structured_invocation_with_metadata_context(
                self.registry,
                context,
                &tokens,
            )?
        };
        (invocation.facts.lowering_hook == Some(tcl_registry::hooks::LoweringHookId::Dict))
            .then_some(())?;
        let argument = invocation
            .facts
            .arg_roles
            .iter()
            .find_map(|&(ordinal, role)| {
                let argument = invocation.facts.argument_offset + usize::from(ordinal);
                (role == ArgRole::VarWrite
                    && invocation.effective.written_argument(argument) == Some(1))
                .then_some(argument)
            })?;
        let name = invocation
            .original_variable_name_operand(argument)?
            .as_str()
            .to_owned();
        let safe_on_uninit = tokens
            .source_binding
            .as_ref()
            .and_then(crate::command_binding::SourceInvocationBinding::invocation_realm)
            .and_then(|realm| {
                invocation.with_metadata_schema(self.registry, context, realm, |schema| {
                    let query = context.context().authoring_query();
                    Some(schema.semantics.safe_on_uninit.is_none_or(|allowed| {
                        tcl_dialect::model::surface_admits(allowed, Some(&query))
                    }))
                })
            })
            .unwrap_or(false);
        Some((name, safe_on_uninit))
    }

    /// Lower `dict` subcommands.
    pub(super) fn lower_dict(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let sub = &args[0];
        let sub_args = &args[1..];

        match sub.as_str() {
            // registry-axis-ok: subcommands — dict's iterating and scoping subcommands; until the resolved invocation's selected subcommand is read
            "for" | "map" if sub_args.len() >= 3 => {
                let Some(var_names) = crate::registry_invocation::original_name_role_values(
                    self.registry,
                    &self.cmd_tokens(seg),
                    1,
                    ArgRole::LoopVarList,
                ) else {
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
                // text.
                if body_tok.is_none() || !self.can_lower_body_word(seg, body_idx + 1) {
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
                        // full argument list.
                        list_braced: self.cmd_tokens(seg).arg_is_braced_literal(2),
                    }],
                    body,
                    body_span: body_tok.map_or(seg.span, |t| self.script_word_span(t)),
                    // registry-axis-ok: subcommands — dict's iterating and scoping subcommands; until the resolved invocation's selected subcommand is read
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
            // registry-axis-ok: subcommands — dict's iterating and scoping subcommands; until the resolved invocation's selected subcommand is read
            "update" | "with" => Statement::Barrier {
                span: seg.span,
                reason: format!("dict {sub}"),
                command: seg.name().into(),
                canonical_command: None,
                args: args.to_vec(),
                tokens: Some(self.cmd_tokens(seg)),
            },

            _ => {
                let output = self.original_dict_write_name(seg);
                Statement::Call {
                    span: seg.span,
                    command: seg.name().into(),
                    canonical_command: None,
                    args: args.to_vec(),
                    defs: output.iter().map(|(name, _)| name.clone()).collect(),
                    reads: vec![],
                    reads_own_defs: output.is_some(),
                    safe_on_uninit: output.is_some_and(|(_, safe)| safe),
                    tokens: Some(self.cmd_tokens(seg)),
                    foreach_groups: None,
                }
            }
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

    /// Whole written script operand, including its closing delimiter.
    /// IR body ranges use this convention even though lexer token spans exclude
    /// nonempty closers. List-form switch elements already supply whole words.
    pub(super) fn script_word_span(&self, token: &tcl_lexer::Token) -> Span {
        tcl_lexer::word_span_at(&self.source, token.span)
    }

    /// Analysis can enter a frozen body with an exact retained source inventory.
    /// Backend compilation still requires its separately authored inline grammar.
    pub(super) fn can_lower_body_word(&self, seg: &SegmentedCommand, word: usize) -> bool {
        super::seg_word_is_static_braced(seg, word)
            || (!self.target.is_bytecode()
                && seg.argv.get(word).is_some_and(|token| {
                    self.source_bindings.as_ref().is_some_and(|bindings| {
                        bindings.executed_script_for_word(token.span).is_some()
                    })
                }))
    }

    fn authored_child_for_token(
        &self,
        tok: &tcl_lexer::Token,
    ) -> Option<(
        std::sync::Arc<crate::registry_invocation::OriginalAuthoredSourceBodyScope>,
        tcl_lexer::NativeWord,
    )> {
        if let Some(scope) = self.authored_source_scope.clone() {
            self.structured_source_parent.as_deref().and_then(|parent| {
                let original = scope.original_words(parent, self.registry)?;
                let body = original
                    .iter()
                    .find(|word| word.span().start() == tok.span.start())?;
                Some((scope.for_plain_body(parent, body)?, body.clone()))
            })
        } else if self.allows_standalone_registry_metadata() {
            self.structured_source_parent.as_deref().and_then(|parent| {
                let grammar = crate::registry_invocation::original_authored_source_structure(parent, self.registry)?
                    .into_closed_source_grammar()?;
                let body = (0..parent.words().len().checked_sub(1)?)
                    .filter_map(|argument| grammar.original_body_word(argument))
                    .find(|word| word.span().start() == tok.span.start())?;
                Some((crate::registry_invocation::OriginalAuthoredSourceBodyScope::capture_plain_root(parent, body, &self.module)?, body.clone()))
            })
        } else {
            None
        }
    }

    /// Lower the executed operand value in its retained source instance.
    /// Escapes and substitutions never receive estimated document offsets.
    pub(super) fn lower_body_from_tok(
        &mut self,
        text: &str,
        tok: Option<&tcl_lexer::Token>,
        _namespace: &str,
    ) -> Script {
        let Some(tok) = tok else {
            return Script::new();
        };
        let authored_child = self.authored_child_for_token(tok);
        if let Some((child, body)) = authored_child {
            let Some(span) = crate::registry_invocation::original_written_script_body_span(&body)
            else {
                return super::over_depth_script(tok.span.start(), text.len());
            };
            let Some(source) = body
                .image()
                .try_text()
                .ok()
                .and_then(|text| text.get(span.as_range()))
                .map(str::to_owned)
            else {
                return super::over_depth_script(tok.span.start(), text.len());
            };
            let Some(namespace) = child.namespace().advisory_key() else {
                return super::over_depth_script(tok.span.start(), text.len());
            };
            let previous_scope = self.authored_source_scope.replace(child.clone());
            let previous_readonly = std::mem::replace(
                &mut self.readonly_authored_body,
                crate::lowering::SourceBodyMode::ReadOnly,
            );
            let mut script = self.lower_body(&source, span.start(), &namespace);
            self.authored_source_scope = previous_scope;
            self.readonly_authored_body = previous_readonly;
            script.namespace_context = Some(Box::new(child.namespace().clone()));
            return script;
        }
        if self.readonly_authored_body.is_read_only() {
            return super::over_depth_script(tok.span.start(), text.len());
        }
        if let Some(script) = self
            .source_bindings
            .as_ref()
            .and_then(|bindings| bindings.executed_script_for_word(tok.span))
            .cloned()
        {
            return self.lower_retained_script(script);
        }
        // The retained parent owns the argument ordinal and compiler word
        // span. Its shared owner joins the Native whole extent independently;
        // a cooked body never borrows an equal-text written site.
        if let Some(source) = self.structured_source_parent.as_deref().and_then(|parent| {
            parent
                .source_binding
                .as_ref()?
                .original_body_source_for_word(parent, tok.span, self.config)
        }) {
            return self.lower_retained_script(source);
        }
        self.lower_source_word_body(text, tok)
    }

    fn lower_source_word_body(&mut self, text: &str, tok: &tcl_lexer::Token) -> Script {
        let Some(origin) = self
            .source_bindings
            .as_ref()
            .and_then(|bindings| bindings.source_origin())
            .cloned()
        else {
            return super::over_depth_script(tok.span.start(), text.len());
        };
        let span = tcl_lexer::word_span_at(&self.source, tok.span);
        let Some(raw) = self.source.get(span.as_range()) else {
            return super::over_depth_script(tok.span.start(), text.len());
        };
        let Some(commands) = self.segment_source(
            raw,
            span.start(),
            super::SourceSegmentationPurpose::NestedScript,
        ) else {
            return super::over_depth_script(tok.span.start(), text.len());
        };
        let Some(command) = commands.first() else {
            return Script::new();
        };
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_bytes_with_channel(
                self.source.as_bytes(),
                self.current_source_channel(),
            ),
            self.config,
            command,
        );
        let Some(word) = tokens.words().first() else {
            return Script::new();
        };
        let crate::registry_invocation::EffectiveInvocationWord::Literal(value) =
            crate::registry_invocation::effective_invocation_word(
                word,
                self.config.escapes,
                WordValueRules::from_config(&self.config),
            )
        else {
            return super::over_depth_script(tok.span.start(), text.len());
        };
        let script = tcl_names::origin::ExecutedScriptSource::from_word(
            tcl_names::command::CommandAllocationSite {
                source: origin,
                offset: span.start(),
            },
            0,
            word,
            &value,
            self.config,
        );
        self.lower_retained_script(script)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod switch_span_tests;

#[cfg(test)]
mod original_body_word_caller_tests;

#[cfg(test)]
mod control_receiver_data_tests;

#[cfg(test)]
mod original_dict_name_tests;

#[cfg(test)]
mod retained_expression_lexer_configuration_tests;
