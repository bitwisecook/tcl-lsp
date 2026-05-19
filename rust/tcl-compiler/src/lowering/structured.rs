//! Structured IR lowering for control-flow commands.
//!
//! Each method converts a segmented command into its corresponding
//! structured IR statement (`If`, `For`, `While`, `Foreach`, `Catch`,
//! `Try`, `Switch`, `dict` subcommands).

use tcl_lexer::{Lexer, SourceMap, Span, TokenType};

use crate::expr_parser::parse_expr;
use crate::ir::{ForeachIterator, IfClause, Script, Statement, SwitchArm, SwitchMode, TryHandler};
use crate::naming::normalise_var_name;
use crate::segmenter::SegmentedCommand;

use crate::segmenter::word_piece;

use super::{parse_param_names, Lowerer};

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

/// Parse switch options, returning `(first_non_option_index, mode, nocase)`.
fn parse_switch_options(args: &[String]) -> (usize, SwitchMode, bool) {
    let mut i = 0;
    let mut mode = SwitchMode::Exact;
    let mut nocase = false;
    while i < args.len() && args[i].starts_with('-') {
        match args[i].as_str() {
            "--" => {
                i += 1;
                break;
            }
            "-glob" => mode = SwitchMode::Glob,
            "-regexp" => mode = SwitchMode::Regexp,
            "-nocase" => nocase = true,
            _ => {}
        }
        i += 1;
    }
    (i, mode, nocase)
}

impl Lowerer<'_> {
    // ── if ────────────────────────────────────────────────────────

    /// Lower `if cond body ?elseif cond body ...? ?else body?`.
    pub(super) fn lower_if(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();

        if args.is_empty() {
            return Self::barrier(seg, "malformed if");
        }

        let mut clauses = Vec::new();
        let mut else_body = None;
        let mut else_span = None;
        let mut i = 0;
        // C38c: reachability tracking. Once a clause's condition
        // folds to a static `true`, every later clause + the
        // ``else`` branch is dead. A clause whose own condition
        // is a static `false` is dead this iteration but not
        // necessarily later — track per-clause via the
        // ``dead_code_depth`` counter.
        let mut later_clauses_dead = false;

        while i < args.len() {
            if args[i] == "elseif" {
                i += 1;
                continue;
            }
            if args[i] == "else" {
                if i + 1 >= args.len() {
                    return Self::barrier(seg, "malformed if else clause");
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
            clauses.push(IfClause {
                condition: parse_expr(&args[cond_idx], None),
                condition_span: cond_tok.map_or(seg.span, |t| t.span),
                body,
                body_span: body_tok.map_or(seg.span, |t| t.span),
            });
            // Static-true condition latches the dead-code flag so
            // remaining clauses + the else branch are suppressed.
            if matches!(static_cond, Some(true)) {
                later_clauses_dead = true;
            }
            i += 1;
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

    // ── for ───────────────────────────────────────────────────────

    /// Lower `for init cond next body`.
    pub(super) fn lower_for(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();

        if args.len() < 4 || arg_tokens.len() < 4 {
            return Self::barrier(seg, "malformed for");
        }
        if !(arg_single[0] && arg_single[1] && arg_single[2] && arg_single[3]) {
            return Self::barrier(seg, "for with dynamic arguments");
        }

        let init = self.lower_body_from_tok(&args[0], Some(&arg_tokens[0]), namespace);
        let next = self.lower_body_from_tok(&args[2], Some(&arg_tokens[2]), namespace);
        let body = self.lower_body_from_tok(&args[3], Some(&arg_tokens[3]), namespace);

        Statement::For {
            span: seg.span,
            init,
            init_span: arg_tokens[0].span,
            condition: parse_expr(&args[1], None),
            condition_span: arg_tokens[1].span,
            next,
            next_span: arg_tokens[2].span,
            body,
            body_span: arg_tokens[3].span,
            raw_args: args.to_vec(),
        }
    }

    // ── while ─────────────────────────────────────────────────────

    /// Lower `while cond body`.
    pub(super) fn lower_while(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
        let args = seg.args();
        let arg_tokens = seg.arg_tokens();
        let arg_single = seg.arg_single_token();

        if args.len() < 2 || arg_tokens.len() < 2 {
            return Self::barrier(seg, "malformed while");
        }
        if !(arg_single[0] && arg_single[1]) {
            return Self::barrier(seg, "while with dynamic arguments");
        }

        let body = self.lower_body_from_tok(&args[1], Some(&arg_tokens[1]), namespace);

        Statement::While {
            span: seg.span,
            condition: parse_expr(&args[0], None),
            condition_span: arg_tokens[0].span,
            body,
            body_span: arg_tokens[1].span,
            raw_args: args.to_vec(),
        }
    }

    // ── foreach / lmap ────────────────────────────────────────────

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

        if args.len() < 3 || args.len() % 2 == 0 {
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

        Statement::Foreach {
            span: seg.span,
            iterators,
            body,
            body_span: body_tok.map_or(seg.span, |t| t.span),
            is_lmap,
            raw_args: args.to_vec(),
            is_dict_iteration: false,
        }
    }

    /// Lower `foreachLine varName filename body` (Tcl 9.0+, TIP 670)
    /// as a single-iterator [`Statement::Foreach`] so variables
    /// assigned inside the body propagate to the enclosing scope —
    /// matching plain `foreach`'s lattice behaviour rather than the
    /// opaque [`Statement::Barrier`] treatment used for generic
    /// stdlib procs.  Mirrors `core/compiler/lowering.py` after
    /// PR #433.
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
    /// treat this IR as a real list-iteration `foreach`.**  Today
    /// the Rust path emits no runtime instructions from
    /// [`Statement::Foreach`] (codegen happens via the Python WASM /
    /// bytecode pipeline, which dispatches on the command name
    /// before lowering); if a Rust runtime codegen is added in the
    /// future, it must detect `raw_args[0] == "foreachLine"` (or
    /// equivalent) before treating this as a list iteration.  The
    /// Python lowerer carries the same invariant.
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
        }
    }

    // ── catch ─────────────────────────────────────────────────────

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
        // Mirrors upstream commit ``342d4c7a`` (PR #331).
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

    // ── try ───────────────────────────────────────────────────────

    /// Lower `try body ?on|trap matchArg varList handlerBody ...? ?finally finallyBody?`.
    pub(super) fn lower_try(&mut self, seg: &SegmentedCommand, namespace: &str) -> Statement {
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

                let var_names = parse_param_names(var_list);
                let result_var = var_names.first().map(|v| normalise_var_name(v).to_owned());
                let options_var = var_names.get(1).map(|v| normalise_var_name(v).to_owned());

                let handler_body = self.lower_body_from_tok(&args[i + 3], handler_tok, namespace);

                handlers.push(TryHandler {
                    kind: keyword.clone(),
                    match_arg,
                    var_name: result_var,
                    options_var,
                    body: handler_body,
                    body_span: handler_tok.map_or(seg.span, |t| t.span),
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

    // ── switch ────────────────────────────────────────────────────

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
            if pair.body_text == "-" {
                arms.push(SwitchArm {
                    pattern: pair.pattern.clone(),
                    pattern_span: pair.pattern_span,
                    body: None,
                    body_span: None,
                    fallthrough: true,
                });
                continue;
            }

            let body_tok = pair.body_arg_idx.and_then(|idx| arg_tokens.get(idx));
            let body = self.lower_body_from_tok(&pair.body_text, body_tok, namespace);

            if pair.pattern == "default" && pair_idx == pairs.len() - 1 {
                default_body = Some(body);
                default_span = pair.body_span;
            } else {
                arms.push(SwitchArm {
                    pattern: pair.pattern.clone(),
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

        let (mut i, mode, nocase) = parse_switch_options(args);

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
            if elements.len() % 2 != 0 {
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
            // Multi-arg form: remaining args are pattern body pairs.
            let remaining = args.len() - i;
            if remaining % 2 != 0 {
                return Self::barrier(seg, "switch odd pattern count");
            }
            while i + 1 < args.len() {
                let pattern = args[i].clone();
                let pattern_span = arg_tokens.get(i).map_or(seg.span, |t| t.span);
                let body_text_inner = args[i + 1].clone();
                let body_tok_idx = i + 1;
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
        }
    }

    // ── dict ──────────────────────────────────────────────────────

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
                }
            }

            "set" | "unset" | "append" | "lappend" | "incr" if !sub_args.is_empty() => {
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

            "update" | "with" => Statement::Barrier {
                span: seg.span,
                reason: format!("dict {sub}"),
                command: seg.name().into(),
                canonical_command: None,
                args: args.to_vec(),
                tokens: Some(Self::cmd_tokens(seg)),
            },

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

    // ── Helpers ───────────────────────────────────────────────────

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
        let _m = lower_to_ir("switch $x { a {puts a} b {puts b} }", &reg());
        // Single braced body may not be fully parsed yet, but multi-arg works:
        let m2 = lower_to_ir("switch $x a {puts a} b {puts b}", &reg());
        assert!(matches!(
            &m2.top_level.statements[0],
            Statement::Switch { mode, .. } if *mode == SwitchMode::Exact
        ));
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
        // for the runtime's eval-fallback.  Mirrors upstream
        // commit ``31f5357f`` (PR #341).
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
        // ``eval_catch`` evaluates the substituted value (Mirrors
        // upstream commit ``342d4c7a`` / PR #331).
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
