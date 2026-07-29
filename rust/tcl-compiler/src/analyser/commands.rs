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

//! Central command dispatch.
//!
//! Walks one segmented Tcl command and routes it through the
//! per-command handlers. Which handler family owns a command form is
//! registry data — the [`tcl_registry::hooks::AnalyserHookId`] stamped
//! on its `CommandSpec` / `SubCommand` — so the dispatch is a single
//! typed `match` ([`Analyser::dispatch_analyser_hook`]); adding a new
//! handler means adding a hook variant and stamping the spec.

use std::sync::OnceLock;

use tcl_core_types::DiagCode;
use tcl_lexer::{Lexer, LexerConfig, SourceMap, Span, Token, TokenType};
use tcl_registry::{AppendedArity, ArgRole, CommandRegistry};

use crate::depth_guard::MAX_BRACKET_TEXT_DEPTH;
use crate::parsing::syntax::descend::{descend_command, descend_token};
use crate::parsing::syntax::segment::segments_from_tree;
use crate::segmenter::SegmentedCommand;

use super::state::Analyser;
use super::types::{CodeFix, Diagnostic, Severity};

/// A command head collected from a `[...]` substitution / expression scan,
/// ready to push as a `command_invocations` entry:
/// `(name, head-span, argc, callback_arity, ensemble_subcommand_candidate)`.
/// `argc` is the nested call's statically-known argument count (`None` when
/// `{*}`-expanded); `callback_arity` is `Some` only for an
/// `ArgRole::CommandPrefix` callback head (so the callback arity check runs)
/// and `None` for an ordinary call head. `ensemble_subcommand_candidate` is
/// `Some((word, span))` when the head's first actual argument is a static,
/// non-`{*}`-expanded word — a *candidate* subcommand
/// [`Analyser::push_collected_heads`] checks against
/// `ensemble_subcommand_targets` once the head's own resolved name is known
/// (issue #923 idx 106: a `[widget make hello]` nested call needs the same
/// subcommand-reference recording a top-level `widget make hello` call
/// already gets from [`Analyser::record_ensemble_subcommand_invocation`]).
type CollectedHead = (
    String,
    Span,
    Option<usize>,
    Option<AppendedArity>,
    Option<(String, Span)>,
);

/// Maximum nested-body recursion depth for [`Analyser::analyse_body`].
/// `analyse_body` ↔ `process_command` ↔ `dispatch_body_arguments`
/// recurse one frame group per braced-body nesting level; generated /
/// minified Tcl and machine-emitted iRules can nest deeply enough to
/// overflow the stack — an uncatchable SIGABRT that crashes the LSP
/// diagnostics worker and the `tcl diag` / `lint` / `validate` CLIs. At
/// the cap we stop descending into further nested bodies (the diagnostics
/// already collected stand); no real source nests anywhere near this.
const MAX_BODY_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(256);

/// The borrowed word-level view of one command, threaded into the
/// dispatch-site diagnostic emitter ([`Analyser::emit_dispatch_site_diagnostics`]).
/// Bundling these co-varying slices keeps the emitter under the argument
/// limit; it destructures them back into locals so the per-code emitter
/// calls read unchanged.
struct DispatchSite<'a> {
    cmd_name: &'a str,
    args: &'a [String],
    arg_tokens: &'a [Token],
    arg_single: &'a [bool],
    arg_expand_in: &'a [bool],
    cmd_tok: Token,
    scope_path: &'a [usize],
}

/// Shared core registry standing in for [`Analyser::registry`] when a
/// handler runs outside an `analyse*` entry point (unit harnesses drive
/// handlers on a bare `Analyser::new()`, which never populates the
/// dialect-aware registry).  Built once; the core `tcl` pack carries
/// every stamped [`tcl_registry::hooks::AnalyserHookId`], so hook
/// resolution behaves identically to an analyse-time run.
fn fallback_registry() -> &'static CommandRegistry {
    static FALLBACK: OnceLock<CommandRegistry> = OnceLock::new();
    FALLBACK.get_or_init(CommandRegistry::build_default)
}

/// Parent command for a control-flow keyword that is only valid as an
/// argument *within* a parent command, or `None` for any other name.
fn orphaned_keyword_parent(cmd_name: &str) -> Option<&'static str> {
    match cmd_name {
        "else" | "elseif" | "then" => Some("if"),
        "on" | "trap" | "finally" => Some("try"),
        _ => None,
    }
}

impl Analyser {
    /// Re-segment a body script and dispatch each command at
    /// `scope_path`.
    ///
    /// Used by every body-walking handler (`handle_proc_command`,
    /// `handle_switch_command`, `handle_try_command`,
    /// `handle_catch_command`, etc.).
    ///
    /// Body recursion does **not** use the segmenter's re-segmentation
    /// recovery — that splits a runaway top-level command and only
    /// fires at the top level. The per-command syntax *detectors* (E100 /
    /// E102 stray closers, E201 unterminated `[`, E202 unterminated `"`,
    /// E203 unterminated `{`) do run on every body. Dynamic bodies
    /// (`$body`, `[gen]`) are skipped because they can't be statically
    /// re-segmented.
    ///
    /// `body_depth` is bumped for the duration of the walk so
    /// top-level-only command checks can distinguish nested
    /// invocations.
    pub(super) fn analyse_body(&mut self, body_text: &str, body_tok: Token, scope_path: &[usize]) {
        if body_tok.kind != TokenType::Str {
            return;
        }
        self.body_depth += 1;
        if MAX_BODY_DEPTH.exceeded(self.body_depth) {
            // Stop descending before the recursive body walk overflows the
            // stack — the diagnostics collected up to this nesting level
            // still stand. Report it as a diagnostic (once per walk, not
            // once per nested body past the cap) rather than truncating
            // silently: tclsh's own recursion limit raises a catchable
            // "too many nested evaluations (infinite loop?)" error at this
            // point rather than continuing quietly, and a process abort is
            // never the right failure mode either way (issue #996).
            if !self.structure_only && !self.e207_emitted {
                self.e207_emitted = true;
                self.result.diagnostics.push(Diagnostic {
                    code: DiagCode::E207,
                    span: body_tok.span,
                    message: format!(
                        "nesting depth exceeds the analysis limit ({} levels) — \
                         diagnostics for this body and anything nested inside it are not \
                         collected",
                        MAX_BODY_DEPTH.0
                    ),
                    severity: Severity::Error,
                    fixes: Vec::new(),
                });
            }
            self.body_depth -= 1;
            return;
        }
        let base_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        // Absolute byte offset at which this body region ends. The E202 /
        // E203 detectors test "reaches end of region" against this, not the
        // whole document — the body's tokens are absolute spans into
        // `self.source`, but a runaway `"` / `{` only swallows to the body's
        // own end.
        let region_end = base_offset as usize + body_text.len();
        let body_commands = crate::segmenter::segment_commands_with_offset_and_config(
            body_text,
            base_offset,
            self.lexer_config(),
        );
        let total = body_commands.len();
        let mut cmd_idx: usize = 0;
        while cmd_idx < total {
            let cmd_ref = &body_commands[cmd_idx];
            if cmd_ref.argv.is_empty() {
                cmd_idx += 1;
                continue;
            }
            if cmd_ref.is_partial {
                // An unterminated `"` / `{` emits the precise E202 /
                // E203 (with a closing-delimiter fix) ahead of the generic
                // E200, matching the top-level `walk_commands_top_level`
                // branch. Stolen-close-brace detection ⇒ E103
                // (brace partials only); otherwise the generic E200 fires so
                // the user still sees a parse-error diagnostic.
                let brace_partial = matches!(
                    cmd_ref.partial_delimiter,
                    Some(crate::segmenter::UnclosedDelimiter::Brace)
                );
                if !(self.emit_unterminated_delimiter_diagnostics(cmd_ref, region_end)
                    || brace_partial && self.detect_stolen_close_brace(cmd_ref))
                {
                    self.emit_partial_command_diagnostic(cmd_ref);
                }
                cmd_idx += 1;
                continue;
            }
            // Run the E100 (stray `]`) / E102 (stray `}`) token
            // checks on every analysed body, not just the top
            // level. Run on the *original* token stream before
            // ``recover_stray_close_bracket`` repairs the clone,
            // matching the top-level loop's ordering.  Token spans
            // are absolute into the full document, so the full
            // ``self.source`` is the right slice base.
            let stray = super::syntax_checks::stray_closer_diagnostics(
                cmd_ref,
                &self.source,
                self.registry,
                || self.user_command_tail_names(),
            );
            self.result.diagnostics.extend(stray);
            // E201 (unterminated `[`) inside a body — `proc p {} { set y
            // [foo }`.  The CST auto-closes the bracket so the command isn't
            // flagged `is_partial`, but the source carries no real `]`, so
            // it would otherwise go unreported.  Run the same E201
            // detector as `emit_syntax_recovery_diagnostics` at the
            // top level (the top-level ghost-recovery doesn't reach
            // into body scripts).
            let e201 = super::syntax_checks::unterminated_bracket_diagnostics(
                cmd_ref,
                &self.source,
                &self.recovery_known_commands,
            );
            self.result.diagnostics.extend(e201);
            // E202 (unterminated `"`) / E203 (unterminated `{`) inside a
            // body — `proc p {} { set x "\n puts hi }`.  The brace word the
            // body sits in is balanced, so the body re-segments cleanly and
            // the run-away quote/brace is a non-partial command whose token
            // reaches the body's end.  Run the same E202/E203 detector
            // as `emit_syntax_recovery_diagnostics` at the top level,
            // which the top-level ghost-recovery doesn't reach into body
            // scripts.
            self.emit_unterminated_delimiter_diagnostics(cmd_ref, region_end);
            let mut cmd = cmd_ref.clone();
            // Repair stray ``]`` (missing ``[``) so downstream
            // handlers see the intended argv shape before dispatch.
            self.recover_stray_close_bracket(&mut cmd);
            // Splice orphaned switch case pairs when ``{`` was
            // forgotten.  The returned count is added to ``cmd_idx``
            // so we skip past the consumed orphans.
            let consumed = self.recover_missing_open_brace(&mut cmd, &body_commands, cmd_idx);
            // ``# noqa`` directives in the preceding-comment
            // attribute to this command's line range — same
            // shape as the top-level loop.
            if let Some(line_offsets) = self.line_offsets.as_deref() {
                super::utils::apply_preceding_noqa(
                    &cmd,
                    line_offsets,
                    &mut self.result.suppressed_lines,
                );
            }
            self.process_command(
                &cmd.texts,
                &cmd.argv,
                &cmd.single_token_word,
                cmd.expand_word.as_deref().unwrap_or(&[]),
                scope_path,
            );
            self.emit_w216_brace_then_paren(&cmd);
            // Record every `$var` substitution in arg positions so
            // `VarDef.references` carries the read spans the LSP
            // providers consume.
            self.record_arg_var_reads(&cmd, scope_path);
            cmd_idx += 1 + consumed;
        }
        self.body_depth -= 1;
    }

    /// Returns `true` when `cmd_name` is hidden in the enclosing safe
    /// interpreter and the caller must skip the command entirely.
    /// Extracted from [`Self::process_command`] to keep that function
    /// within the line budget.
    fn safe_interp_visibility_gate(&mut self, cmd_name: &str, cmd_tok: Token) -> bool {
        // Safe-interpreter visibility gate (issue #945 fault 7): inside a
        // safe interpreter's evaluation body, a command whose registry spec
        // is safe-hidden (`Traits::SAFE_INTERP_HIDDEN`) — or was
        // `interp hide`-den — and not re-exposed raises `invalid command
        // name` in C *before* any effect happens.  Flag it (W129) and skip
        // the command entirely: no invocation record, no handler dispatch,
        // no source / package / definition edges built from a call that
        // never executes.  The set membership is registry data; no command
        // name appears here.
        let Some(ctx) = self.safe_interp_stack.last() else {
            return false;
        };
        let bare = cmd_name.trim_start_matches(':');
        let spec_hidden = ctx.base_hidden
            && self.registry.and_then(|r| r.get(bare)).is_some_and(|spec| {
                spec.traits
                    .contains(tcl_registry::Traits::SAFE_INTERP_HIDDEN)
            });
        let hidden =
            (spec_hidden || ctx.hidden_extra.contains(bare)) && !ctx.exposed.contains(bare);
        if !hidden {
            return false;
        }
        if !self.structure_only {
            self.result.diagnostics.push(super::types::Diagnostic {
                code: tcl_core_types::DiagCode::W129,
                span: cmd_tok.span,
                message: format!(
                    "'{cmd_name}' is hidden in this safe interpreter — the call \
                     raises `invalid command name` unless it is exposed or \
                     invoked via `interp invokehidden`"
                ),
                severity: super::types::Severity::Warning,
                fixes: Vec::new(),
            });
        }
        true
    }

    /// A flattened snapshot of `self.safe_interp_stack`'s *top* entry, for
    /// [`super::per_item::DeferredBody::safe_interp_ctx`] (issue #1001
    /// follow-up) — see that field's doc for the full rationale. Called
    /// wherever a proc/method/apply body is deferred for `analyse_per_item`'s
    /// isolated second pass, so the visibility context is available for
    /// [`super::per_item::analyse_proc_body_isolated`] to restore.
    pub(super) fn safe_interp_ctx_snapshot(&self) -> Option<(bool, Vec<String>, Vec<String>)> {
        let ctx = self.safe_interp_stack.last()?;
        let mut hidden_extra: Vec<String> = ctx.hidden_extra.iter().cloned().collect();
        hidden_extra.sort_unstable();
        let mut exposed: Vec<String> = ctx.exposed.iter().cloned().collect();
        exposed.sort_unstable();
        Some((ctx.base_hidden, hidden_extra, exposed))
    }

    /// Issue #1001's two `process_command`-level extensions to
    /// [`Self::safe_interp_visibility_gate`], combined into one call so
    /// `process_command` stays within its line budget:
    ///
    /// 1. `{*}[list HEAD arg …]` as this command's own (expand-marked)
    ///    head: Tcl expands the list `list` builds into this statement's
    ///    own argv, so the command's *effective* head is `HEAD`, not the
    ///    substitution text `cmd_name` holds — which never matches a
    ///    registry name, so the literal-head gate silently no-ops for this
    ///    shape. Resolved through the same `[list …]` command-quoting idiom
    ///    [`Self::check_list_quoted_deferred_head`] uses; stops exactly as
    ///    the literal-head gate does when the effective head is hidden.
    /// 2. [`Self::check_deferred_call_safe_interp_hiding`] for this
    ///    command's own `Body` / `LambdaLiteral` / `CommandPrefix`-role
    ///    argument positions (the pervasive `[list apply …]`
    ///    deferred-command idiom) — never itself a reason to stop
    ///    processing *this* command, since each such argument is an
    ///    independent deferred call, not this command's own head.
    ///
    /// Returns `true` when the caller must stop (this command's own
    /// effective head was hidden), mirroring
    /// `safe_interp_visibility_gate`'s contract. A no-op outside a tracked
    /// safe-interpreter context.
    fn check_indirect_hiding(
        &mut self,
        argv_texts: &[String],
        arg_tokens_in: &[Token],
        arg_expand_in: &[bool],
        scope_path: &[usize],
    ) -> bool {
        if self.safe_interp_stack.is_empty() {
            return false;
        }
        let cmd_name = argv_texts[0].as_str();
        let head_tok = arg_tokens_in[0];
        if arg_expand_in.first().copied().unwrap_or(false)
            && head_tok.kind == TokenType::Cmd
            && self.check_list_quoted_deferred_head(head_tok, cmd_name, scope_path)
        {
            return true;
        }
        let args = argv_texts.get(1..).unwrap_or(&[]);
        let arg_tokens = arg_tokens_in.get(1..).unwrap_or(&[]);
        if self.check_ensemble_redirect_hiding(cmd_name, args, arg_tokens, scope_path) {
            return true;
        }
        self.check_deferred_call_safe_interp_hiding(cmd_name, args, arg_tokens, scope_path);
        false
    }

    /// Resolve a call through a tracked `namespace ensemble create|configure
    /// ... -map {sub target ...}` redirect (issue #1001 follow-up — the same
    /// theme as issue #979's interprocedural call-site gap, but for W129
    /// rather than constant-propagation seeding, and confirmed via a
    /// dedicated investigation to be a genuinely distinct, previously
    /// untracked gap): `cmd_name` isn't itself a hidden registry name — it's
    /// the ensemble's own command name (`myens`) — but if it resolves to a
    /// tracked ensemble (`self.ensemble_command_maps`, populated by
    /// [`Self::handle_namespace_ensemble`]) and `args[0]` (the subcommand) is
    /// one of its mapped entries, the call's *effective* target is whatever
    /// that entry names, which might be hidden.
    ///
    /// Returns `true` when the resolved target was hidden (a W129 was
    /// already pushed), mirroring `safe_interp_visibility_gate`'s contract.
    /// A no-op outside a tracked safe interpreter, when `cmd_name` doesn't
    /// resolve to a tracked ensemble, or when `args[0]` isn't one of its
    /// mapped subcommands.
    fn check_ensemble_redirect_hiding(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if self.safe_interp_stack.is_empty() {
            return false;
        }
        let Some(sub) = args.first() else {
            return false;
        };
        let resolved = self.resolve_command_qualified_name(cmd_name, scope_path);
        let target = self
            .ensemble_command_maps
            .get(&resolved)
            .and_then(|m| m.get(sub).cloned());
        let (Some(target), Some(tok)) = (target, arg_tokens.first().copied()) else {
            return false;
        };
        self.safe_interp_visibility_gate(&target, tok)
    }

    /// Extend [`Self::safe_interp_visibility_gate`] through the `[list HEAD
    /// …]` command-quoting idiom (issue #1001): the literal-head gate above
    /// only ever sees a command whose head is written directly, so the
    /// pervasive deferred-command idiom (`package ifneeded … [list apply
    /// {…} $dir]`, `-command [list apply {…} $x]`, `after idle [list apply
    /// {…} $x]`, `trace add … command [list apply {…} $x]`) — building a
    /// callback around a dynamic value with `list` rather than writing a
    /// literal `apply {…} $dir` — is entirely invisible to it.
    ///
    /// Checked only for this command's own `Body` / `LambdaLiteral` /
    /// `CommandPrefix`-role argument positions — the exact `deferred_role`
    /// gate the semantic-token highlighter's list-quoted-lambda recognition
    /// uses (`deferred_role_arg_starts` in `tcl-lsp-core`'s
    /// `semantic_tokens.rs`, from the codex-review follow-up to #954) — so a
    /// `[list apply {…} value]` sitting in ordinary data (`set data [list
    /// apply {…} value]`, no role at that position) is never treated as a
    /// call; only a position the registry already marks as later
    /// invoked/sourced is. See
    /// `docs/kcs/kcs-issue-apply-lambda-body-not-highlighted-via-list-quoting.md`
    /// decision rule 4 for why that highlighting-only precision is
    /// deliberately widened here rather than reused unchanged: a missed
    /// W129 (false negative) is the worse failure mode for a security
    /// diagnostic, unlike a spurious highlight.
    ///
    /// A no-op outside a tracked safe-interpreter context (the overwhelming
    /// common case) — this never runs, and never has any side effect (no
    /// new scope, no new diagnostic of any other kind), for ordinary code.
    fn check_deferred_call_safe_interp_hiding(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if self.safe_interp_stack.is_empty() {
            return;
        }
        let Some(registry) = self.registry else {
            return;
        };
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        let mut indices: Vec<usize> = [
            ArgRole::Body,
            ArgRole::LambdaLiteral,
            ArgRole::CommandPrefix,
        ]
        .into_iter()
        .flat_map(|role| registry.arg_indices_for_role(cmd_name, &arg_strs, role))
        .collect();
        indices.sort_unstable();
        indices.dedup();
        for idx in indices {
            let (Some(tok), Some(text)) = (arg_tokens.get(idx), args.get(idx)) else {
                continue;
            };
            // A whole-argument `[…]` substitution is the only shape `list`
            // command-quoting can take here — a braced or bareword value at
            // a deferred-call position is either a literal script (handled
            // by the ordinary `Body`-role `analyse_body` walk) or a plain
            // `CommandPrefix` head already resolved by
            // `record_command_prefix_invocations`, neither of which needs
            // this resolution.
            if tok.kind != TokenType::Cmd {
                continue;
            }
            self.check_list_quoted_deferred_head(*tok, text, scope_path);
        }
    }

    /// Resolve one `[list HEAD arg1 arg2 …]`-shaped deferred-call argument:
    /// gate `HEAD` for safe-interpreter visibility (issue #1001), recursing
    /// into an `apply` lambda body via [`Self::handle_apply_command`] — the
    /// SAME handler a literal `apply {…} $x` call dispatches to — when
    /// `HEAD` resolves to it, so a hidden command nested inside the lambda
    /// (the reported repro's `source` call) is caught by the ordinary,
    /// unmodified gate the recursion's own `process_command` calls hit.
    /// `self.safe_interp_stack` is untouched by this recursion (only
    /// `interp eval` pushes/pops it), so the enclosing safe interpreter's
    /// visibility context is inherited automatically — exactly as it is for
    /// a directly-written `apply {…} $x` call already inside the same body.
    ///
    /// Returns `true` when `HEAD` itself was hidden (a W129 was already
    /// pushed) so a caller checking its *own* effective head (the `{*}[list
    /// HEAD …]` argv-expansion shape) can stop further processing the same
    /// way the literal-head gate does.
    fn check_list_quoted_deferred_head(
        &mut self,
        tok: Token,
        text: &str,
        scope_path: &[usize],
    ) -> bool {
        let Some(registry) = self.registry else {
            return false;
        };
        let Some(seg) =
            crate::signature_scan::command_prefix::list_quoted_command_segment(registry, tok, text)
        else {
            return false;
        };
        let head = seg.texts[1].clone();
        let head_tok = seg.argv[1];
        if self.safe_interp_visibility_gate(&head, head_tok) {
            return true;
        }
        let rest_args = seg.texts.get(2..).unwrap_or(&[]);
        if matches!(
            self.resolve_analyser_hook(&head, rest_args),
            Some(tcl_registry::hooks::AnalyserHookId::Apply)
        ) {
            let rest_tokens = seg.argv.get(2..).unwrap_or(&[]);
            self.handle_apply_command(rest_args, rest_tokens, scope_path);
        }
        false
    }

    /// Process a single segmented command.
    ///
    /// Walks `args` against every handler and stops at the first
    /// match. Non-matching commands fall through silently —
    /// they're either unknown (the W123 emitter handles reporting)
    /// or registry-known commands that don't need analyser-side
    /// intervention (the IR pass does the heavy lifting).
    ///
    /// `argv_texts` and `arg_tokens` parallel each other:
    /// `argv_texts[0]` is the command name, `argv_texts[1..]`
    /// the arguments. `arg_tokens[0]` is the command-name token.
    /// `single_token_word` is parallel to argv and indicates
    /// whether each word is a single atomic token (used by
    /// ``handle_set_command`` for the const-string heuristic).
    ///
    /// Simple-command arity (E002 / E003) is emitted here via
    /// [`Self::emit_arity_diagnostics`]; the candidates are
    /// flushed post-walk by [`Self::flush_arity_diagnostics`].
    pub fn process_command(
        &mut self,
        argv_texts: &[String],
        arg_tokens_in: &[Token],
        single_token_word: &[bool],
        arg_expand_in: &[bool],
        scope_path: &[usize],
    ) {
        if argv_texts.is_empty() || arg_tokens_in.is_empty() {
            return;
        }
        let cmd_name = argv_texts[0].as_str();
        // Safe-interpreter visibility gate (issue #945 fault 7): a command
        // hidden in the enclosing safe interpreter never executes — see
        // `safe_interp_visibility_gate`'s doc for the full rationale.
        if self.safe_interp_visibility_gate(cmd_name, arg_tokens_in[0]) {
            return;
        }
        let args = if argv_texts.len() > 1 {
            &argv_texts[1..]
        } else {
            &[]
        };
        let arg_tokens = if arg_tokens_in.len() > 1 {
            &arg_tokens_in[1..]
        } else {
            &[]
        };
        let arg_single = if single_token_word.len() > 1 {
            &single_token_word[1..]
        } else {
            &[]
        };
        // Bracket-substitution indirection (issue #1001) invisible to the
        // gate above — see `check_indirect_hiding`'s doc.
        if self.check_indirect_hiding(argv_texts, arg_tokens_in, arg_expand_in, scope_path) {
            return;
        }
        // Record this invocation so the post-walk
        // ``emit_unresolved_command_diagnostics`` (W123) can iterate
        // every command head the analyser visited.  ``inv.range``
        // anchors at the command-head token so the W123 message
        // points at the unresolved name rather than the whole
        // command line.
        let cmd_tok = arg_tokens_in[0];
        // Structure-only mode (item-tree extraction) skips every diagnostic /
        // cross-feature recording pass below — they don't affect the declared
        // proc / class / alias / ensemble structure and are the bulk of the
        // per-command cost.  The structural handlers further down still run, so
        // `file_decls` is identical to a full `analyse` (gated by the
        // `file_decls_corpus` corpus test).
        if !self.structure_only {
            let resolved = self.resolve_command_qualified_name(cmd_name, scope_path);
            // Argument count for cross-file arity checking.  A `{*}`-
            // expanded argument makes the runtime count unknown, so record `None`
            // and let arity checking skip conservatively.
            let arg_count = if arg_expand_in.iter().skip(1).copied().any(|e| e) {
                None
            } else {
                Some(args.len())
            };
            let resolved_cmd = resolved.clone();
            self.result.command_invocations.push(
                crate::signature_scan::types::SignatureCommandInvocation {
                    name: cmd_name.to_string(),
                    range: cmd_tok.span,
                    resolved_qualified_name: Some(resolved),
                    resolution_candidates: Vec::new(),
                    argc: arg_count,
                    callback_arity: None,
                    callback_baked_args: 0,
                    indirect: false,
                    rename_safe: true,
                    existence_probe: false,
                    is_mathfunc_call: false,
                },
            );

            // `<ensemble> <subcommand> …` — record an additional, existence
            // -probed `CommandInvocation` for the subcommand word so
            // references, rename, call-hierarchy, and go-to-definition see
            // through a static `namespace ensemble create -map`/
            // `-subcommands` mapping the same way they already see through
            // an `interp alias` (issue #923 idx 106).
            self.record_ensemble_subcommand_invocation(
                &resolved_cmd,
                args,
                arg_tokens_in,
                arg_expand_in,
            );

            // iRules ``call PROC ARG...`` — record an additional
            // ``CommandInvocation`` for the target proc so that
            // references, rename, and call-hierarchy see through the
            // indirection.
            self.record_irules_call_invocation(cmd_name, args, arg_tokens_in, scope_path);

            // Walk every argument's source slice for ``[cmd ...]``
            // substitutions and record each nested head as its own
            // ``CommandInvocation``.
            self.record_nested_invocations_from_args(cmd_name, args, arg_tokens_in, scope_path);

            // Record `ArgRole::CommandPrefix` callback heads (`lsort -command
            // myCompare`, `trace add … cb`) as command invocations too, so
            // find-references / rename / call-hierarchy / code-lens / W123 /
            // callback-arity see the callback exactly like a direct call.
            self.record_command_prefix_invocations(
                cmd_name, args, arg_tokens, arg_single, scope_path,
            );

            // Record `ArgRole::CommandName` arguments (`info body PROC`,
            // `namespace which -command NAME`) — a bare command name held as
            // data — as command invocations too, so the named command is
            // reached by find-references / go-to-definition / rename without
            // any arity check (it is introspected, not called).
            self.record_command_name_invocations(cmd_name, args, arg_tokens, scope_path);

            // Run the per-command syntactic checks on commands nested inside
            // ``[…]`` substitutions — the main walk never descends a
            // substitution (it treats `[cmd …]` as a value), so a command
            // like `set fh [open "|$cmd" r]` or `set x [string index abc 99]`
            // would otherwise escape the security / bounds / arity / style
            // families entirely.  Re-runs the per-command checks on each
            // descended substitution command.
            self.run_nested_command_diagnostics(arg_tokens_in, scope_path);

            // Run the per-command syntactic + EXPR checks on commands nested
            // inside a ``[…]`` substitution of a *braced expression*
            // argument (`if { [matchclass …] }`, `while { [done $x] }`).
            // The bare-`Cmd` walk above never enters a braced `Str` expr
            // arg, so those substitution commands would otherwise escape
            // every per-command check (IRULE2001/2002, W100, …).
            self.run_nested_expr_diagnostics(cmd_name, args, arg_tokens, scope_path);

            // Record variable-as-command and
            // command-substitution-as-command call sites so the
            // post-walk W307 / W308 emitters can resolve them.
            self.record_var_or_cmd_command_site(
                cmd_tok,
                arg_expand_in.first().copied().unwrap_or(false),
                args,
                arg_tokens,
                arg_expand_in.get(1..).unwrap_or(&[]),
                scope_path,
            );

            // W125 (orphaned control-flow keyword) and IRULE5005 (direct
            // iRules-proc call without `call`) — both key off whether the
            // command head resolves to a user proc, so they share one
            // resolution.
            self.emit_proc_resolution_diagnostics(cmd_name, args, cmd_tok, scope_path);

            // Record TclOO instance creation (`set v [Cls new]`,
            // `Cls create inst`) so the LSP providers can resolve
            // ``$v method`` / ``inst method`` call sites to the
            // object's class.
            self.record_instance_creation(cmd_name, args);

            // When the constructor's class head is a `$var` reference
            // instead of a literal bareword, defer to the flow-sensitive
            // value model (issue #923 idx 121) rather than dropping the
            // instance's class entirely.
            self.record_pending_instance_class_site(cmd_name, args, arg_tokens);

            // Generic EXPR-argument walk via the command registry's
            // ``ArgRole::Expr``.  Picks up the condition arg of
            // ``if`` / ``elseif`` / ``while`` / the cond+next slots
            // of ``for`` / the body of ``expr`` / etc.  Currently
            // hosts the W110 (``==``/``!=`` vs ``eq``/``ne``)
            // emitter; future EXPR-role checks slot in here.
            //
            // Run *before* the early-returning handlers
            // (``handle_for_command`` / ``handle_foreach_command`` /
            // ``handle_switch_command`` / ``handle_catch_command`` /
            // ``handle_try_command``) so EXPR-role args on those
            // commands aren't skipped — none of those handlers
            // process EXPR args themselves (they own *body*
            // recursion only), so this can't double-fire.
            self.dispatch_expr_arguments(cmd_name, args, arg_tokens, cmd_tok);

            // Dispatch-site diagnostic emitters (W302 / W001 / E004 / W101
            // / W304 / W004 / E002-E003).  Extracted from this function so
            // it stays within the line budget; see the method for the
            // per-code rationale and ordering.  Run before the
            // early-returning handlers so option-bearing / body-owning
            // commands still get checked.
            self.emit_dispatch_site_diagnostics(&DispatchSite {
                cmd_name,
                args,
                arg_tokens,
                arg_single,
                arg_expand_in,
                cmd_tok,
                scope_path,
            });
        } // end `if !self.structure_only`

        self.dispatch_command_handlers(cmd_name, args, arg_tokens, arg_single, cmd_tok, scope_path);
    }

    /// Record an iRules `call PROC ARG...`'s target as its own
    /// `CommandInvocation` so references / rename / call-hierarchy see
    /// through the indirection. Extracted from [`Self::process_command`]
    /// so it stays within the line budget.
    fn record_irules_call_invocation(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens_in: &[Token],
        scope_path: &[usize],
    ) {
        if cmd_name != "call" || self.dialect() != "f5-irules" {
            return;
        }
        let (Some(target_name), Some(target_tok)) = (args.first(), arg_tokens_in.get(1).copied())
        else {
            return;
        };
        let resolved = self.resolve_command_qualified_name(target_name, scope_path);
        self.result.command_invocations.push(
            crate::signature_scan::types::SignatureCommandInvocation {
                name: target_name.clone(),
                range: target_tok.span,
                resolved_qualified_name: Some(resolved),
                resolution_candidates: Vec::new(),
                // iRules `call PROC ...` indirection — arity not
                // cross-file-checked here; skip conservatively.
                argc: None,
                callback_arity: None,
                callback_baked_args: 0,
                indirect: false,
                rename_safe: true,
                existence_probe: false,
                is_mathfunc_call: false,
            },
        );
    }

    /// Typed per-command dispatch for [`Self::process_command`].
    ///
    /// Which handler family owns a command form is registry data — the
    /// [`AnalyserHookId`] stamped on the `CommandSpec` (or, for the
    /// subcommand-shaped families like `namespace eval` / `dict for` /
    /// `interp alias`, on the `SubCommand`) — so the dispatch is one
    /// typed `match`, not a chain of name-guarded calls.  The
    /// early-return families stop the walk exactly as their `true`
    /// return used to; the void families fall through to the shared
    /// tail: the registry-role-driven handlers that consider every
    /// command (`VarWrite` bindings, symbol definers, the tcllib
    /// `::import` wrapper idiom) and the generic `ArgRole::Body`
    /// recursion.
    fn dispatch_command_handlers(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        cmd_tok: Token,
        scope_path: &[usize],
    ) {
        if self.dispatch_analyser_hook(cmd_name, args, arg_tokens, arg_single, cmd_tok, scope_path)
        {
            return;
        }

        // Any command with registry `VarWrite`-role args (`lassign`, `scan`,
        // `regexp`, `regsub`, `gets`, `binary scan`, `vwait`, …) writes
        // results into named variable arguments; bind them so
        // completion/hover/definition see the destructured / captured names.
        self.handle_var_binding_command(cmd_name, args, arg_tokens, scope_path);
        // Registry symbol-definer commands (`tcltest::test NAME …`) contribute a
        // lightweight named definition to the outline.  Void handler — it only
        // records the symbol; the body still recurses via the generic
        // `ArgRole::Body` walk below.
        self.handle_defines_symbol(cmd_name, args, arg_tokens, arg_single, scope_path);
        // The tcllib `<NS>::import <alias>` wrapper idiom — recognised by
        // the call's own `::import` tail, not a registry name.
        self.handle_tcllib_import_wrapper(cmd_name, cmd_tok, args, scope_path);

        // Generic body recursion via the command registry's
        // `ArgRole::Body`.  Picks up `if` / `while` / `when` /
        // `eval` / `uplevel` / `subst` / etc. — every command whose
        // registry spec marks an argument index as `BODY`.  The
        // early-return hook arms above already consumed the commands
        // that own their body walk (proc, oo::class, oo::define,
        // namespace eval, foreach, for, switch, catch, try), so this
        // loop only fires for the rest.
        //
        // For `when EVENT { body }` the iRules dialect spec
        // marks arg 1 as BODY; set `current_event` for the body
        // walk so race-detection diagnostics see the event name.
        self.dispatch_body_arguments(cmd_name, args, arg_tokens, arg_single, scope_path);
    }

    /// Resolve the [`AnalyserHookId`] for a command head, mirroring the
    /// retired per-handler guards exactly: the head must be the spec's
    /// own spelling — a `::`-qualified spelling of a **bareword** global
    /// command (`::proc`, `::namespace`, …) resolves no hook (the literal
    /// guards never matched one, and `CommandRegistry::get`'s leading-`::`
    /// fallback must not widen *that* dispatch) — and a subcommand word
    /// must match its `SubCommand` name exactly.
    ///
    /// A **namespaced** registry spelling (`tcl::OptProc`, `oo::define`) is
    /// different: real corpus code commonly writes it fully qualified
    /// (`::tcl::OptProc`, issue #923 idx 90), and `get`'s single-`::`-strip
    /// fallback resolves that to the exact same spec either way — an exact
    /// match, never the naive tail-matching the guard above exists to rule
    /// out. Allowed only when the bareword tail *itself* still contains a
    /// `::` (so `::proc` → `proc`, no embedded `::`, stays blocked; `::tcl
    /// ::OptProc` → `tcl::OptProc`, embedded `::`, is allowed) — hooks
    /// stamped before this distinction existed keep their exact prior
    /// behaviour; only a namespaced spec's own qualified spelling gains
    /// coverage.
    ///
    /// Outside an `analyse*` run (unit harnesses drive handlers on a
    /// bare `Analyser::new()`) the shared core registry stands in for
    /// the stashed dialect-aware one.
    pub(in crate::analyser) fn resolve_analyser_hook(
        &self,
        cmd_name: &str,
        args: &[String],
    ) -> Option<tcl_registry::hooks::AnalyserHookId> {
        if let Some(bare) = cmd_name.strip_prefix("::")
            && !bare.contains("::")
        {
            return None;
        }
        let registry = self.registry.unwrap_or_else(fallback_registry);
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        registry
            .resolve_call(
                cmd_name,
                &arg_strs,
                tcl_registry::prelude::DialectSet::empty(),
            )
            .and_then(|resolved| resolved.analyser_hook)
    }

    /// The single typed `match` over the resolved [`AnalyserHookId`].
    ///
    /// Returns `true` when the command was consumed by an early-return
    /// family (the caller stops, skipping the shared tail), `false`
    /// when the walk should continue — either a void family ran, or no
    /// hook (and no definition-grammar definer) matched.
    #[allow(
        clippy::too_many_lines,
        reason = "exhaustive registry-hook dispatch (one arm per AnalyserHookId \
                  variant, so a new variant is a compile error until wired); \
                  splitting hurts readability, per classify_side_effects"
    )]
    fn dispatch_analyser_hook(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        cmd_tok: Token,
        scope_path: &[usize],
    ) -> bool {
        use tcl_registry::hooks::AnalyserHookId as Hook;
        let Some(hook) = self.resolve_analyser_hook(cmd_name, args) else {
            // No stamped family — the definition-grammar-driven definers
            // (TclOO metaclass create, snit::type/widget, itcl::class) get
            // their chance, then the tracked-interpreter-handle dispatch
            // (`sandbox eval { … }`, `cmd_name` never a registry name so it
            // can never reach a stamped hook the way literal `interp eval`
            // does).  Their registry/analysis-state conditions are disjoint
            // from every stamped hook and from each other (pinned by
            // tcl-registry's analyser-hook drift tests for the registry
            // ones), so running them only on the hookless path preserves
            // the old chain's order.
            return self.handle_oo_class_command(cmd_name, args, arg_tokens, scope_path)
                || self.handle_snit_type_command(cmd_name, args, arg_tokens, scope_path)
                || self.handle_itcl_class_command(cmd_name, args, arg_tokens, scope_path)
                || self.handle_interp_handle_eval_command(cmd_name, args, arg_tokens, scope_path);
        };
        match hook {
            // Early-return families: the handler owns the whole command
            // (including its body walk) when it returns `true`.
            Hook::Proc => self.handle_proc_command(args, arg_tokens, arg_single, scope_path),
            Hook::OptProc => self.handle_opt_proc_command(args, arg_tokens, arg_single, scope_path),
            // `interp eval path { … }` — the child interpreter's script is
            // analysed in an isolated scope; a `{}`/multi-word/dynamic shape
            // falls through to the generic body walk in the current scope.
            Hook::InterpEval => self.handle_interp_eval_command(args, arg_tokens, scope_path),
            Hook::OoDefine => self.handle_oo_define_command(cmd_name, args, arg_tokens, scope_path),
            Hook::NamespaceEval => self.handle_namespace_eval_command(args, arg_tokens, scope_path),
            // uplevel #0 { body } — opens a global-frame child scope so
            // the body's locals don't leak into the enclosing proc's
            // variable set.  Only the `#0` form is consumed; other
            // levels fall through to the generic body recursion.
            Hook::Uplevel => self.handle_uplevel_command(args, arg_tokens, scope_path),
            Hook::Foreach => self.handle_foreach_command(args, arg_tokens, scope_path),
            Hook::For => self.handle_for_command(args, arg_tokens, scope_path),
            Hook::Switch => self.handle_switch_command(args, arg_tokens, scope_path),
            Hook::Catch => self.handle_catch_command(args, arg_tokens, scope_path),
            Hook::Try => self.handle_try_command(args, arg_tokens, scope_path),
            // apply {{params} body} — owns its body walk (binds params,
            // analyses element 1) so the generic `ArgRole::Body`
            // recursion never mis-reads the parameter list as a command.
            Hook::Apply => self.handle_apply_command(args, arg_tokens, scope_path),

            // Void families: run the handler(s), then fall through to
            // the shared tail.
            Hook::InterpCreate => {
                self.handle_interp_create_command(args);
                false
            }
            Hook::InterpDelete => {
                self.handle_interp_delete_command(args);
                false
            }
            Hook::InterpHide => {
                self.handle_interp_hide_command(args);
                false
            }
            Hook::InterpExpose => {
                self.handle_interp_expose_command(args);
                false
            }
            Hook::Set => {
                self.handle_set_command(args, arg_tokens, arg_single, scope_path);
                self.handle_auto_path_set(args, arg_tokens);
                false
            }
            Hook::Variable => {
                self.handle_variable_command(args, arg_tokens, scope_path);
                false
            }
            Hook::Global => {
                self.handle_global_command(args, arg_tokens, scope_path);
                false
            }
            Hook::Incr => {
                self.handle_incr_command(args, arg_tokens, scope_path);
                false
            }
            Hook::Append => {
                self.handle_append_lappend_command(args, arg_tokens, scope_path);
                false
            }
            Hook::Lappend => {
                self.handle_append_lappend_command(args, arg_tokens, scope_path);
                self.handle_auto_path_lappend(args, arg_tokens);
                false
            }
            Hook::Upvar => {
                self.handle_upvar_command(args, arg_tokens, scope_path);
                false
            }
            Hook::NamespaceUpvar => {
                self.handle_namespace_upvar_command(args, arg_tokens, scope_path);
                false
            }
            Hook::DictFor => {
                self.handle_dict_for_command(args, arg_tokens, scope_path);
                false
            }
            Hook::DictUpdate => {
                self.handle_dict_update_command(args, arg_tokens, scope_path);
                false
            }
            Hook::DictWith => {
                self.handle_dict_with_command(args, arg_tokens, scope_path);
                false
            }
            Hook::NamespaceEnsemble => {
                self.handle_namespace_ensemble(args, arg_tokens, scope_path);
                false
            }
            Hook::InterpAlias => {
                self.handle_interp_alias(args, scope_path, cmd_tok.span.start());
                false
            }
            Hook::OoObjdefine => self.handle_oo_objdefine(args, arg_tokens, scope_path),
            Hook::PackageRequire => {
                self.handle_package_require(cmd_tok, args, arg_tokens);
                false
            }
            Hook::PackageProvide => {
                self.handle_package_provide(cmd_tok, args);
                false
            }
            Hook::Source => {
                self.handle_source_command(args, arg_tokens, arg_single, scope_path);
                false
            }
            Hook::NamespaceImport => {
                self.handle_namespace_import_command(args, arg_tokens, scope_path);
                false
            }
            Hook::NamespaceExport => {
                self.handle_namespace_export_command(args, arg_tokens, scope_path);
                false
            }
            Hook::NamespacePath => {
                self.handle_namespace_path_command(args, scope_path);
                false
            }
            Hook::NamespaceUnknown => {
                self.handle_namespace_unknown_command(args);
                false
            }
            Hook::RegexPatternCapture => {
                self.handle_regex_pattern_capture(cmd_name, args, arg_tokens, scope_path);
                false
            }
            // ``load`` unconditionally flips ``has_dynamic_providers``:
            // it brings a shared library's commands into the interpreter
            // at runtime, which static W123 unknown-command analysis can
            // never predict.
            Hook::Load => {
                self.result.has_dynamic_providers = true;
                false
            }
            // A *static* ``rename OLD NEW`` is recorded precisely by
            // ``handle_rename`` (``NEW`` resolves to whatever ``OLD``
            // denoted, including its arity) — only a genuinely *dynamic*
            // rename (``rename $x y`` / ``rename x [y]``) falls back to
            // the same conservative flag as ``load``, matching
            // ``command_binding.rs``'s wildcard-collapse convention for
            // the identical shape.
            Hook::Rename => {
                if self.handle_rename(
                    args,
                    arg_tokens,
                    arg_single,
                    scope_path,
                    cmd_tok.span.start(),
                ) {
                    self.result.has_dynamic_providers = true;
                }
                false
            }
        }
    }

    /// Emit the two diagnostics that key off whether the command head
    /// resolves to a user proc:
    ///
    /// - **W125** (Warning) — an orphaned control-flow keyword
    ///   (`else` / `elseif` / `then` / `on` / `trap` / `finally`) used as a
    ///   standalone command.  This almost always means a misplaced newline
    ///   split its parent `if` / `try` (`}\nelse {` instead of `} else {`).
    ///   Suppressed when a user proc *or* a registry command of the same
    ///   name shadows the keyword.
    /// - **IRULE5005** (Error) — a user proc invoked directly inside an
    ///   iRules event body, where Tcl-on-TMM requires `call PROC ARGS`.
    ///   Ships a `call PROC`-rewrite [`CodeFix`].
    ///
    /// The proc is resolved once and shared between both checks.
    pub(super) fn emit_proc_resolution_diagnostics(
        &mut self,
        cmd_name: &str,
        args: &[String],
        cmd_tok: Token,
        scope_path: &[usize],
    ) {
        let resolves_to_proc = self.resolve_proc_call(cmd_name, scope_path).is_some();

        if !resolves_to_proc
            && let Some(parent) = orphaned_keyword_parent(cmd_name)
            && self
                .registry
                .as_ref()
                .is_none_or(|r| r.get(cmd_name).is_none())
        {
            self.result.diagnostics.push(Diagnostic {
                code: DiagCode::W125,
                span: cmd_tok.span,
                message: format!(
                    "\"{cmd_name}\" used as standalone command — should be part of \
                     \"{parent}\" (check for misplaced newline)"
                ),
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }

        if resolves_to_proc
            && cmd_name != "call"
            && self.current_event.is_some()
            && self.dialect() == "f5-irules"
        {
            let suffix = if args.is_empty() {
                String::new()
            } else {
                format!(" {}", args.join(" "))
            };
            self.result.diagnostics.push(Diagnostic {
                code: DiagCode::Irule5005,
                span: cmd_tok.span,
                message: format!(
                    "iRules procs must be invoked with 'call': call {cmd_name}{suffix}"
                ),
                severity: Severity::Error,
                fixes: vec![CodeFix {
                    span: cmd_tok.span,
                    new_text: format!("call {cmd_name}"),
                    description: format!("Use 'call {cmd_name}'"),
                }],
            });
        }
    }

    /// Dispatch-site diagnostic emitters, run from
    /// [`Self::process_command`] before the early-returning handlers so
    /// option-bearing / body-owning commands still get checked.
    ///
    /// - **W302** (`catch` without a result variable) — fires before
    ///   the early-returning `handle_catch_command`.
    /// - **W001** (unknown subcommand on a `SubcommandSig` command) —
    ///   before `handle_namespace_eval_command` so `namespace foo` is
    ///   flagged.
    /// - **E004** (malformed `if`) — dispatched generically off the
    ///   resolved spec's `clause_shape_check` hook, not off `cmd_name`,
    ///   so `if` is the trigger today only because it is the one
    ///   command carrying that hook.
    /// - **W101** (`eval` with substituted args) — before body-walk
    ///   dispatch so the `ArgRole::Body` recursion into the `eval`
    ///   body still runs.
    /// - **W304** (missing `--` option terminator) — driven by the
    ///   registry's option-terminator profile.
    /// - **W004** (option not available in the active dialect).
    /// - **E002 / E003** (arity) — collected here and flushed
    ///   post-walk by [`Self::flush_arity_diagnostics`].
    /// - **W143** (direct call into a private `::tcl::` implementation
    ///   namespace) — dialect-independent, registry-driven.
    fn emit_dispatch_site_diagnostics(&mut self, site: &DispatchSite<'_>) {
        let DispatchSite {
            cmd_name,
            args,
            arg_tokens,
            arg_single,
            arg_expand_in,
            cmd_tok,
            scope_path,
        } = *site;
        if cmd_name == "catch" {
            self.emit_w302_catch_no_result_var(args, cmd_tok, arg_tokens, arg_single);
        }
        self.emit_w001_unknown_subcommand(
            cmd_name,
            args,
            cmd_tok,
            arg_tokens,
            arg_expand_in,
            scope_path,
        );
        self.emit_w002_disabled_command(cmd_name, cmd_tok, scope_path);
        if let Some(checker) = self
            .registry
            .as_ref()
            .and_then(|r| r.get(cmd_name))
            .and_then(|spec| spec.clause_shape_check)
        {
            self.emit_e004_clause_shape_diagnostic(cmd_name, checker, args, cmd_tok, arg_tokens);
        }
        if let Some(gate) = self
            .registry
            .as_ref()
            .and_then(|r| r.get(cmd_name))
            .and_then(|spec| spec.context_gate)
        {
            self.emit_w142_context_gate(gate, args, cmd_tok);
        }
        self.emit_w101_eval_string_concat(cmd_name, args, arg_tokens, arg_single);
        // W102 / W103 / W300 / W301 / W309 / W312 security-injection
        // checks.
        self.emit_w102_subst_injection(cmd_name, args, arg_tokens);
        self.emit_w103_open_pipeline(cmd_name, args, arg_tokens, arg_single);
        self.emit_w300_source_variable(cmd_name, args, arg_tokens);
        self.emit_w309_eval_subst_double_decode(cmd_name, args, arg_tokens);
        self.emit_w301_uplevel_injection(cmd_name, args, arg_tokens, arg_single);
        self.emit_w312_interp_eval_injection(cmd_name, args, arg_tokens, arg_single);
        self.emit_w303_redos(cmd_name, args, arg_tokens);
        self.emit_w306_literal_expected(cmd_name, args, arg_tokens);
        // W310 runs for every command (it scans args for credential
        // option flags), so it takes no cmd_name guard.
        self.emit_w310_hardcoded_credentials(cmd_name, args, arg_tokens);
        // W143: direct call into a private `::tcl::` implementation
        // namespace. Dialect-independent (prefix-level only, issue #988).
        self.emit_w143_private_tcl_namespace(cmd_name, cmd_tok);
        // IRULE2002: deprecated iRules command (f5-irules only).
        self.emit_irule2002_deprecated_command(cmd_name, cmd_tok);
        // IRULE2001: deprecated `matchclass` (f5-irules only).  Fires
        // alongside IRULE2002 at the same command-head span.
        self.emit_irule2001_matchclass(cmd_name, arg_tokens, cmd_tok);
        // IRULE1003 / 1004 / 2101 / 4001 / 4003 / 5001 / 6001 —
        // analyser-level iRules event-context checks (f5-irules only).
        self.emit_irules_event_checks(cmd_name, args, arg_tokens, cmd_tok);
        // TK1001 / TK1002 / TK1003 — Tk-dialect widget + geometry checks
        // (tk dialect only); the TK1001 conflict is flushed post-walk.
        self.emit_tk_checks(cmd_name, args, arg_tokens, cmd_tok);
        self.emit_w212_name_vs_value(cmd_name, args, arg_tokens, scope_path);
        self.emit_w104_append_list(cmd_name, args, arg_tokens, arg_expand_in, cmd_tok);
        self.emit_w106_unbraced_switch_body(cmd_name, args, arg_tokens);
        self.emit_w311_encoding_mismatch(cmd_name, args, arg_tokens);
        self.emit_w200_binary_format_modifiers(cmd_name, args, arg_tokens);
        self.emit_w121_invalid_subnet_mask(args, arg_tokens);
        self.emit_w108_non_ascii(arg_tokens);
        // W240 / W241 loop-termination + W230 / W232 index-bounds.
        let loop_diags = super::bounds_checks::loop_termination_diagnostics(
            cmd_name,
            args,
            arg_tokens,
            self.registry,
        );
        self.result.diagnostics.extend(loop_diags);
        let idx_diags = super::bounds_checks::list_index_diagnostics(cmd_name, args, arg_tokens);
        self.result.diagnostics.extend(idx_diags);
        let lset_diags =
            super::bounds_checks::lset_index_diagnostics(cmd_name, args, arg_tokens, &self.source);
        self.result.diagnostics.extend(lset_diags);
        let str_diags = super::bounds_checks::string_index_diagnostics(cmd_name, args, arg_tokens);
        self.result.diagnostics.extend(str_diags);
        self.emit_w127_closed_value_args(cmd_name, args, arg_tokens, cmd_tok);
        self.emit_w127_closed_option_values(cmd_name, args, arg_tokens, cmd_tok);
        self.emit_w304_missing_option_terminator(cmd_name, args, cmd_tok, arg_tokens);
        self.emit_w217_unset_option_only(cmd_name, args, arg_tokens);
        self.emit_w004_dialect_invalid_option(
            cmd_name,
            args,
            arg_tokens,
            arg_expand_in.get(1..).unwrap_or(&[]),
            scope_path,
        );
        // W135 / W136 — command/option needs a newer package version than the
        // resolved `package require` floor (buffered, decided post-walk).
        self.record_version_gate_sites(cmd_name, args, arg_tokens, cmd_tok);
        // W138 — format/scan %-string conversions gated behind a Tcl
        // release (buffered, decided post-walk — §6 argument-DSL rung).
        self.record_dsl_format_sites(cmd_name, args, arg_tokens);
        self.emit_arity_diagnostics(
            cmd_name,
            args,
            arg_tokens,
            arg_expand_in,
            cmd_tok,
            scope_path,
        );
    }

    /// Generic EXPR-argument walk via the command registry's
    /// `ArgRole::Expr`.  Invokes the W100 / W110 / W003 / W114
    /// emitters on each EXPR-role argument.  For `expr`, multi-arg
    /// invocations are joined with spaces before the W110 walk.
    fn dispatch_expr_arguments(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        cmd_tok: Token,
    ) {
        // An earlier unconditional user `proc` of the same name shadows
        // the builtin at this call site (Tcl resolves the proc, not
        // `::expr`/`::if`/`::while`/`::for` etc.) — same rule W002 uses
        // for an ordinary disabled-command shadow. Vanishingly rare for
        // these particular control-flow keywords in practice, but cheap
        // to guard once here for every EXPR-role emitter at once, rather
        // than duplicating the check per diagnostic.
        let qualified = crate::naming::normalise_qualified_name(cmd_name);
        if super::utils::proc_shadows_call(&self.result.all_procs, &qualified, cmd_tok.span.start())
        {
            return;
        }
        let Some(registry) = self.registry else {
            return;
        };
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        let mut indices = registry.arg_indices_for_role(
            cmd_name,
            &arg_strs,
            tcl_registry::arg_role::ArgRole::Expr,
        );
        if indices.is_empty() {
            return;
        }
        indices.sort_unstable();
        // Whether this command concatenates its whole argument tail into one
        // expression (the registry's `EXPR_CONCATENATES_ARGS` trait — `expr`).
        // Resolved before the emitters below so the registry borrow ends
        // ahead of the `&mut self` calls.
        let concatenates_args = registry.get(cmd_name).is_some_and(|s| {
            s.traits
                .contains(tcl_registry::Traits::EXPR_CONCATENATES_ARGS)
        });

        // W100: unbraced expression argument. Runs for every
        // EXPR-role form, including the `expr 1 + 2` multi-word case
        // handled by the early return below.
        self.emit_w100_unbraced_expr(cmd_name, args, arg_tokens);

        // Special-case the whole-tail expression command
        // (`concatenates_args` — `expr`): when the user wrote multiple
        // arguments (``expr $a == "x"`` instead of the more common
        // ``expr {$a eq "x"}``), anchor W110 at the full argument
        // token range and parse the joined arguments — the
        // *substituted* word values, with quote delimiters already
        // stripped by Tcl's word splitting.  So ``expr $a == "x"`` parses
        // as ``$a == x`` where ``x`` is a bareword, not an ``ExprString``,
        // and W110 (string ``==``) does NOT fire — matching what `expr`
        // actually receives at runtime.  W003 gets its own emitter for
        // this shape (`emit_w003_dialect_invalid_expr_words`): a gated
        // operator here is always its own standalone word, already at
        // a tight span, with no offset remapping needed.
        if concatenates_args && args.len() > 1 && !arg_tokens.is_empty() {
            let span = tcl_lexer::Span::new(
                arg_tokens[0].span.start(),
                arg_tokens[arg_tokens.len() - 1].span.end(),
            );
            let expr_text = args.join(" ");
            self.emit_w110_string_eq_ne(
                &expr_text,
                span,
                &super::diagnostics::W110Anchor::JoinedWords {
                    args,
                    tokens: arg_tokens,
                },
            );
            self.emit_w003_dialect_invalid_expr_words(args, arg_tokens, &expr_text);
            return;
        }

        for idx in indices {
            if let (Some(text), Some(tok)) = (args.get(idx), arg_tokens.get(idx)) {
                self.emit_w110_string_eq_ne(
                    text,
                    tok.span,
                    &super::diagnostics::W110Anchor::ArgToken(*tok),
                );
                // W003 anchors on the argument's inner content (delimiters
                // stripped via `content_offset`), not the raw token span —
                // it re-slices `self.source` directly so its operator
                // offsets always land on real bytes, byte-for-byte, even
                // when `text` itself has been reconstructed/canonicalised
                // by the segmenter (e.g. a quoted `"$x lt $y"` argument).
                let content_span = tcl_lexer::Span::new(
                    tok.span.start() + u32::from(tok.content_offset),
                    tok.span.end(),
                );
                self.emit_w003_dialect_invalid_expr_operator(content_span);
                self.emit_expr_function_dialect_diagnostics(*tok);
                self.emit_w114_redundant_nested_expr(text, tok.span);
            }
        }
    }

    /// Generic body recursion via the command registry's
    /// `ArgRole::Body`.  Picks up `if` / `while` / `when` / `eval`
    /// / `uplevel` / `subst` / etc. — every command whose registry
    /// spec marks an argument index as `BODY`.  Sets
    /// ``current_event`` for ``when EVENT { body }`` and bumps
    /// ``conditional_depth`` for ``if`` / ``try``.  Emits W105
    /// before recursing into each body so the unbraced-body
    /// warning fires on the body argument's own range, not on
    /// any nested re-segmentation.
    fn dispatch_body_arguments(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) {
        // Current namespace for the imported-command fallback below. Computed
        // before the immutable `registry` borrow (it needs `&mut self`), and
        // only when imports were recorded — `namespace_from_scope_path` caches.
        let cur_ns = if self.result.namespace_imports.is_empty() {
            String::new()
        } else {
            self.namespace_from_scope_path(scope_path)
        };
        let Some(registry) = self.registry else {
            return;
        };
        let body_args: Vec<&str> = args.iter().map(String::as_str).collect();
        // Body-role resolution stays dialect-scoped *deliberately*: a command
        // that owns a body only in another dialect (e.g. the iRules-only
        // `when`) is, under a plain-tcl dialect, an unknown user command whose
        // braced `{...}` is an ordinary string argument — not a script. So we
        // do NOT recurse into it (and do not fire W123/W002 on its contents).
        // Analyse iRules under the f5-irules dialect, where `when` is a real
        // body-owning command.
        let mut body_indices = registry.arg_indices_for_role(
            cmd_name,
            &body_args,
            tcl_registry::arg_role::ArgRole::Body,
        );
        // Fallback for an *imported* command called by its unqualified name:
        // `namespace import ::tcltest::*` followed by `test name desc { body }`
        // calls the registry's `tcltest::test`, whose body roles only resolve
        // under the qualified name. Re-query through each recorded import so the
        // body walk reaches inside the imported command's body. Conservative:
        // only when the bare name owns no body itself,
        // and only against a namespace explicitly imported in this document.
        // The registry command name that actually owns the body role — usually
        // `cmd_name`, but the qualified target when an import fallback resolved
        // it.  Used to read the scoped-body environment from the same spec.
        let mut body_cmd_owned: Option<String> = None;
        if body_indices.is_empty() && !cmd_name.contains("::") {
            for imp in &self.result.namespace_imports {
                // An import is only in effect where it was made: in its own
                // namespace, or — for an import at global scope — everywhere, via
                // Tcl's unqualified-name fallback to `::`. An import inside
                // `namespace eval ns { … }` must NOT resolve a bare call made in a
                // sibling or parent namespace.
                if imp.ns != cur_ns && imp.ns != "::" {
                    continue;
                }
                let candidate = if let Some(prefix) = imp.pattern.strip_suffix('*') {
                    format!("{prefix}{cmd_name}")
                } else if imp.pattern.rsplit("::").next() == Some(cmd_name) {
                    imp.pattern.clone()
                } else {
                    continue;
                };
                let idxs = registry.arg_indices_for_role(
                    &candidate,
                    &body_args,
                    tcl_registry::arg_role::ArgRole::Body,
                );
                if !idxs.is_empty() {
                    body_indices = idxs;
                    body_cmd_owned = Some(candidate);
                    break;
                }
            }
        }
        if body_indices.is_empty() {
            return;
        }
        // Scoped command environment for this command's body args, if any — the
        // curated command set a `report::defstyle` style script (etc.) exposes.
        // Both the environment pointer and the sibling-definition name index are
        // `Copy`, so extracting them here ends the `registry` borrow before the
        // `&mut self` body recursion below.
        let body_cmd: &str = body_cmd_owned.as_deref().unwrap_or(cmd_name);
        let body_scope: Option<&'static tcl_registry::scoped::ScopedCommandEnv> =
            registry.get(body_cmd).and_then(|s| s.body_scope);
        let sibling_name_idx = if body_scope.is_some_and(|e| e.include_sibling_definitions) {
            registry
                .arg_indices_for_role(body_cmd, &body_args, tcl_registry::arg_role::ArgRole::Name)
                .first()
                .copied()
        } else {
            None
        };
        // A definer that makes its own instances callable inside sibling bodies
        // (a later `report::defstyle` may invoke an earlier style by name):
        // record the defined name so the W123 pass treats it as known there.
        if let (Some(env), Some(ni)) = (body_scope, sibling_name_idx)
            && let Some(name) = args.get(ni)
            && !name.is_empty()
        {
            self.result
                .scoped_sibling_defs
                .entry(env.name)
                .or_default()
                .insert(name.clone());
        }
        let prev_event = self.current_event.clone();
        if cmd_name == "when" && !args.is_empty() {
            self.current_event = Some(args[0].clone());
        }
        let is_conditional = matches!(cmd_name, "if" | "try");
        if is_conditional {
            self.conditional_depth += 1;
        }
        for idx in body_indices {
            if let (Some(body_text), Some(body_tok)) = (args.get(idx), arg_tokens.get(idx).copied())
            {
                let is_single_token = arg_single.get(idx).copied().unwrap_or(false);
                self.dispatch_one_body_argument(
                    cmd_name,
                    body_text,
                    body_tok,
                    is_single_token,
                    scope_path,
                    body_scope,
                );
            }
        }
        if is_conditional {
            self.conditional_depth -= 1;
        }
        if cmd_name == "when" {
            self.current_event = prev_event;
        }
    }

    /// Analyse a single body-role argument: fires `W105`, walks the script
    /// (through a scoped command environment when `body_scope` names one),
    /// and — for a bareword (unbraced) body — also dispatches it as a
    /// zero-arg command call. Split out of [`Self::dispatch_body_arguments`]
    /// purely to keep that function under the line-count lint; the two
    /// always run together, once per body-role index.
    fn dispatch_one_body_argument(
        &mut self,
        cmd_name: &str,
        body_text: &str,
        body_tok: Token,
        is_single_token: bool,
        scope_path: &[usize],
        body_scope: Option<&'static tcl_registry::scoped::ScopedCommandEnv>,
    ) {
        self.emit_w105_unbraced_body(cmd_name, body_text, body_tok, is_single_token);
        // A bareword body (`if {$cond} mymod::foo`, `uplevel 1
        // mymod::qux`) is a single, statically-known zero-arg command
        // call — a legitimate alternative form real Tcl accepts
        // identically to a braced block (the exact shape
        // `emit_w105_unbraced_body` above already exempts from its
        // own warning). `analyse_body` below only ever recurses into
        // a `Str` (braced) body, so such a call was previously
        // invisible to `command_invocations` entirely: found by
        // hover/definition (which resolve independently off the
        // cursor token) but missed by references/rename — silently
        // producing an incomplete rename that breaks the program at
        // the missed call site (differential-audit finding idx 61).
        // Dispatched through the ordinary `process_command` path
        // (not a hand-rolled invocation record) so it gets full
        // treatment: arity checking, W123, nested diagnostics —
        // everything a real call site deserves.
        if body_tok.kind == TokenType::Esc
            && !body_text.trim().contains(char::is_whitespace)
            && !super::diagnostics::helpers::has_substitution(body_text.trim(), &body_tok)
        {
            self.process_command(
                &[body_text.trim().to_string()],
                &[body_tok],
                &[true],
                &[false],
                scope_path,
            );
        }
        // A bare `$var` body (`eval $cmd`, `uplevel #0 $cmd …`) dynamically
        // evaluates $var's value as a script at runtime — its first word is
        // the command actually dispatched, exactly the same "value is a
        // command prefix" shape `{*}$cmd` already gets via `head_expanded`
        // (issue #923 idx 94). `analyse_body` above only ever recurses a
        // literal `Str` body, so this site was previously invisible to
        // `command_invocations` entirely (missed by references/rename,
        // unlike hover/go-to-definition's independent cursor-token walk) —
        // record it the same way `record_var_or_cmd_command_site` records a
        // command's own `$cmd`-headed dispatch, for `settle_const_dispatches`
        // to resolve in the CFG/SSA phase. Guarded to a "pure" reference
        // (`var_name == raw`) so a composite word like `${cmd}Suffix` (a
        // literal-concatenated value, not $cmd's own value) is left alone.
        if body_tok.kind == TokenType::Var {
            let sm = tcl_lexer::SourceMap::new(&self.source);
            let raw = sm.token_text(body_tok);
            let var_name = raw
                .split_once('}')
                .map_or(raw, |(name, _)| name)
                .to_string();
            if var_name == raw {
                let ns = self.command_resolution_namespace(scope_path);
                self.pending_const_dispatches
                    .push(super::state::ConstDispatchSite {
                        var_name,
                        span: body_tok.span,
                        ns,
                        head_expanded: true,
                    });
            }
        }
        // When the body runs in a scoped command environment, record its
        // region (so the post-walk W123 pass and the LSP providers can
        // resolve the scoped heads by position) and push the environment
        // so the in-walk arity / subcommand checks resolve them too.
        if let Some(env) = body_scope {
            let start = body_tok.span.start() + u32::from(body_tok.content_offset);
            self.result
                .scoped_command_regions
                .push(super::types::ScopedBodyRegion {
                    span: tcl_lexer::Span::new(start, body_tok.span.end()),
                    env,
                });
            self.body_scope_stack.push(env);
            self.analyse_body(body_text, body_tok, scope_path);
            self.body_scope_stack.pop();
        } else {
            self.analyse_body(body_text, body_tok, scope_path);
        }
    }

    /// Record each [`tcl_registry::arg_role::ArgRole::CommandPrefix`] callback
    /// head of this call (`lsort -command myCompare`, `trace add … cb`,
    /// `interp alias {} a {} target`) as a `CommandInvocation`, so the
    /// callback is a first-class command reference: find-references, rename,
    /// call-hierarchy, code-lens usage counts, W123 unknown-command, and the
    /// callback-arity check all see it exactly like a direct call.  Only a
    /// literal bareword head is recorded (see
    /// [`crate::signature_scan::command_prefix`]); a dynamic `$cb` / `[..]`
    /// head stays unrecorded so W123 doesn't false-fire.
    fn record_command_prefix_invocations(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        scope_path: &[usize],
    ) {
        let Some(registry) = self.registry else {
            return;
        };
        let mut invs = crate::signature_scan::command_prefix::command_prefix_invocations(
            registry, cmd_name, args, arg_tokens, arg_single,
        );
        // Instance-method dispatch: `$obj method …` / `objName method …` where
        // the receiver's class is a registry-modelled object class and the
        // method declares a command prefix (`$g walk … -command cb`,
        // `$t walkproc … cb`).  The receiver's class comes from the progressive
        // `instance_classes` map (bound by a prior `struct::graph name` /
        // `set g [Class new]`), keyed by the bare handle (leading `$` stripped).
        if let Some(method) = args.first() {
            // The handle is a bare object command (`objName method …`) or a
            // variable dispatch, whose head reconstructs as `$g` or `${g}` — map
            // all three to the `instance_classes` key (the bare name).
            let receiver = cmd_name.strip_prefix('$').map_or(cmd_name, |v| {
                v.strip_prefix('{')
                    .and_then(|b| b.strip_suffix('}'))
                    .unwrap_or(v)
            });
            if let Some(class) = self.result.instance_classes.get(receiver) {
                invs.extend(
                    crate::signature_scan::command_prefix::instance_method_command_prefix_invocations(
                        registry,
                        class,
                        method,
                        args.get(1..).unwrap_or(&[]),
                        arg_tokens.get(1..).unwrap_or(&[]),
                        arg_single.get(1..).unwrap_or(&[]),
                    ),
                );
            }
        }
        for inv in invs {
            let resolved = self.resolve_command_qualified_name(&inv.head, scope_path);
            self.result.command_invocations.push(
                crate::signature_scan::types::SignatureCommandInvocation {
                    name: inv.head,
                    range: inv.span,
                    resolved_qualified_name: Some(resolved),
                    resolution_candidates: Vec::new(),
                    // The legacy direct-call arity path always skips a
                    // callback head (`None`); the callback-arity check reads
                    // `callback_baked_args` (0 for a bareword head, N for a
                    // braced multi-word prefix) + `callback_arity`.
                    argc: None,
                    callback_arity: Some(inv.appended),
                    callback_baked_args: inv.baked,
                    indirect: false,
                    rename_safe: true,
                    existence_probe: false,
                    is_mathfunc_call: false,
                },
            );
        }
    }

    /// Record a **command reference** — a command named as data rather than
    /// invoked with a fixed argument list at this span (an `info body` target,
    /// a `forward` / ensemble `-map` target, an expression math function).
    /// `written` is the head as it appears, `span` its token, `resolved` the
    /// qualified command it denotes; [`Self::finalise_invocation_resolutions`]
    /// recovers the namespace from the `(written, resolved)` pair and settles
    /// the candidate list, so a rename rewrites only the written token.  `argc`
    /// is the call-site argument count for arity checking, or `None` when the
    /// site merely names the command.
    pub(in crate::analyser) fn push_command_reference(
        &mut self,
        written: String,
        span: Span,
        resolved: String,
        argc: Option<usize>,
    ) {
        self.push_command_reference_with_policy(written, span, resolved, argc, false);
    }

    /// [`Self::push_command_reference`] with an explicit existence policy:
    /// `existence_probe: true` records a reference the W123 pass must skip
    /// (the probed command legitimately may not exist — issue #945
    /// fault 9).
    pub(in crate::analyser) fn push_command_reference_with_policy(
        &mut self,
        written: String,
        span: Span,
        resolved: String,
        argc: Option<usize>,
        existence_probe: bool,
    ) {
        self.result.command_invocations.push(
            crate::signature_scan::types::SignatureCommandInvocation {
                name: written,
                range: span,
                resolved_qualified_name: Some(resolved),
                resolution_candidates: Vec::new(),
                argc,
                callback_arity: None,
                callback_baked_args: 0,
                indirect: false,
                rename_safe: true,
                existence_probe,
                is_mathfunc_call: false,
            },
        );
    }

    /// [`Self::push_command_reference`] for an `expr` math-function call —
    /// see
    /// [`crate::signature_scan::types::SignatureCommandInvocation::is_mathfunc_call`].
    /// A dedicated method rather than another positional bool on
    /// [`Self::push_command_reference_with_policy`]: it has exactly one
    /// caller, and a same-typed `existence_probe, is_mathfunc_call` pair
    /// invites a silently-transposed call.
    pub(in crate::analyser) fn push_mathfunc_command_reference(
        &mut self,
        written: String,
        span: Span,
        resolved: String,
        argc: Option<usize>,
    ) {
        self.result.command_invocations.push(
            crate::signature_scan::types::SignatureCommandInvocation {
                name: written,
                range: span,
                resolved_qualified_name: Some(resolved),
                resolution_candidates: Vec::new(),
                argc,
                callback_arity: None,
                callback_baked_args: 0,
                indirect: false,
                rename_safe: true,
                existence_probe: false,
                is_mathfunc_call: true,
            },
        );
    }

    /// Record each [`tcl_registry::arg_role::ArgRole::CommandName`] argument as
    /// a command invocation: a bare command name held as data (`info body
    /// PROC`, `info args PROC`, `info default PROC …`) that navigation must
    /// reach, but which is *not* invoked here — so it carries no call arity.
    /// A dynamic word (`info body $p`) names no static command and is skipped.
    fn record_command_name_invocations(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        let Some(registry) = self.registry else {
            return;
        };
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        // Required-existence references and probe references share the
        // recording; only the existence policy carried on the record
        // differs (a probe never feeds W123 — issue #945 fault 9).
        for (role, probe) in [
            (tcl_registry::arg_role::ArgRole::CommandName, false),
            (tcl_registry::arg_role::ArgRole::CommandNameProbe, true),
        ] {
            for idx in registry.arg_indices_for_role(cmd_name, &arg_strs, role) {
                let (Some(name), Some(tok)) = (args.get(idx), arg_tokens.get(idx)) else {
                    continue;
                };
                if name.is_empty() || crate::naming::is_dynamic_word(name) {
                    continue;
                }
                // A glob pattern probes a *set* of commands, not one exact
                // name — no single reference identity exists, so abstain.
                if probe && name.contains(['*', '?']) {
                    continue;
                }
                let resolved = self.resolve_command_qualified_name(name, scope_path);
                self.push_command_reference_with_policy(
                    name.clone(),
                    tok.span,
                    resolved,
                    None,
                    probe,
                );
            }
        }
    }

    /// `<ensemble> <subcommand> …` — when `resolved_cmd` names a known
    /// ensemble (a key in [`AnalysisResult::ensemble_subcommand_targets`])
    /// and the first actual argument is a static, non-`{*}`-expanded
    /// subcommand word present in that ensemble's map, record a second,
    /// existence-probed [`SignatureCommandInvocation`] for the subcommand
    /// word pointing at its resolved target — the same "referenceable but
    /// never independently callable" shape [`Self::record_command_name_invocations`]
    /// already uses for `CommandNameProbe` (issue #945 fault 9): `make` is
    /// never itself a valid command name (only the pair `widget make`
    /// dispatches), so it must never feed W123. Lets `definition`/`hover`/
    /// `references`/rename/call-hierarchy resolve `widget make` to
    /// `::widget::Make` (issue #923 idx 106) through the same
    /// `resolved_qualified_name`-matching path every other indirection
    /// (alias, rename, iRules `call`) already uses — no separate per
    /// -provider ensemble-aware code needed there.
    ///
    /// [`AnalysisResult::ensemble_subcommand_targets`]: super::types::AnalysisResult::ensemble_subcommand_targets
    /// [`SignatureCommandInvocation`]: crate::signature_scan::types::SignatureCommandInvocation
    fn record_ensemble_subcommand_invocation(
        &mut self,
        resolved_cmd: &str,
        args: &[String],
        arg_tokens_in: &[Token],
        arg_expand_in: &[bool],
    ) {
        // `arg_expand_in[0]` is the command name's own (always-`false`)
        // flag; `arg_expand_in[1]` is the subcommand word's — a `{*}`
        // -expanded subcommand names no static word at all.
        if arg_expand_in.get(1).copied().unwrap_or(false) {
            return;
        }
        let Some(sub_map) = self.result.ensemble_subcommand_targets.get(resolved_cmd) else {
            return;
        };
        let Some(sub) = args.first() else { return };
        if crate::naming::is_dynamic_word(sub) {
            return;
        }
        let Some(target) = sub_map.get(sub).cloned() else {
            return;
        };
        let Some(tok) = arg_tokens_in.get(1) else {
            return;
        };
        // Args *after* the consumed subcommand word — not `args.len()`,
        // which would double-count the subcommand word itself against the
        // target proc's real arity.  `{*}` anywhere in the target's own
        // arguments makes the runtime count unknown, same convention as the
        // head invocation just above.
        let sub_argc = if arg_expand_in
            .get(2..)
            .is_some_and(|rest| rest.iter().any(|&e| e))
        {
            None
        } else {
            Some(args.len() - 1)
        };
        self.push_command_reference_with_policy(sub.clone(), tok.span, target, sub_argc, true);
    }

    /// Walk every argument's source slice for ``[cmd ...]``
    /// substitutions and record each nested head as its own
    /// ``CommandInvocation``.  Extracted from
    /// [`Self::process_command`] for readability — without this,
    /// calls embedded inside argument expressions
    /// (``set x [helper $foo]``, ``puts "got [count $items]"``,
    /// ``if { [HTTP::uri] eq "/foo" }``) aren't tracked, which
    /// breaks workspace usage counts, find-references, rename,
    /// and call-hierarchy.
    fn record_nested_invocations_from_args(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens_in: &[Token],
        scope_path: &[usize],
    ) {
        // Which arguments are *expressions*?  A `[...]` inside a braced
        // expr arg is a real invocation (`if {[acl_ok]} …`), but a `[...]`
        // inside a braced *data* word is literal (`set x {[noeval]}`) — so
        // a braced word is scanned only when it is an `Expr` arg.  A braced
        // *body* arg is covered separately by `analyse_body`.
        let expr_indices: Vec<usize> = self
            .registry
            .as_ref()
            .map(|r| {
                let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
                r.arg_indices_for_role(cmd_name, &arg_strs, ArgRole::Expr)
            })
            .unwrap_or_default();
        // A command whose *name* is itself a substitution (`[x] hi`):
        // descend a `Cmd` head too, since every token including the
        // head can hold a nested invocation (the head's *name* is
        // recorded separately by `process_command`; this records what
        // it substitutes).
        if let Some(head) = arg_tokens_in.first()
            && head.kind == TokenType::Cmd
        {
            self.record_invocations_from_cmd_token(*head, scope_path);
        }
        for (i, arg_tok) in arg_tokens_in.iter().enumerate().skip(1) {
            let arg_start = arg_tok.span.start();
            let arg_end = arg_tok.span.end() as usize;
            let src_len = self.source.len();
            if arg_start as usize >= src_len || arg_end > src_len {
                continue;
            }
            if arg_tok.kind == TokenType::Cmd {
                self.record_invocations_from_cmd_token(*arg_tok, scope_path);
            } else if arg_tok.kind == TokenType::Str {
                // Braced word: scan its `[...]` substitutions only when it
                // is an expression argument (substitutions are then active);
                // a braced data word stays opaque (a non-expr braced word
                // is never walked as a script).
                if expr_indices.contains(&(i - 1)) {
                    self.record_invocations_from_expr_token(*arg_tok, scope_path);
                }
            } else {
                // `Esc` (bareword / quoted): substitutions are active, so
                // scan.  Clone the slice into an owned ``String`` so the
                // helper can take ``&mut self`` without conflicting with the
                // source borrow.
                let arg_src = self.source[arg_start as usize..arg_end].to_string();
                self.record_invocations_from_word_token(*arg_tok, &arg_src, arg_start, scope_path);
            }
        }
    }

    /// Inner: ``Cmd`` (``[…]``) substitution tokens.  Descend the
    /// substitution into a child CST ([`descend_token`]), segment it,
    /// and record *every* inner command's bareword head, recursing
    /// into nested ``[...]``.
    ///
    /// A flat [`first_command_head`] scan would record only the
    /// *first* head of each ``[...]``, dropping ``;``- / newline-
    /// separated commands (`[foo; bar]` → only `foo`); the CST
    /// descent finds them all.
    fn record_invocations_from_cmd_token(&mut self, arg_tok: Token, scope_path: &[usize]) {
        let config = self.lexer_config();
        // Collect the inner heads first (this borrows `self.source`
        // through the `SourceMap`); resolve + push afterwards so the
        // immutable source borrow has ended.
        let (heads, expr_toks) = {
            let sm = Analyser::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            );
            let mut heads: Vec<CollectedHead> = Vec::new();
            let mut expr_toks: Vec<Token> = Vec::new();
            // `arg_tok` is the *merged* argv token.  For a compound word
            // whose first fragment is a `[…]` substitution (`[foo]bar`,
            // `[foo]$x`, `[foo]bar[baz]`), `segments_from_tree` widens the
            // span from the first fragment's start to the *last* fragment's
            // end.  Descending that merged span would re-lex the trailing
            // literal as a script and record a bogus head (`[foo]bar` →
            // `foo]bar`).  Descend each `[…]` fragment instead, walking the
            // unmerged token stream; a single-fragment `[…]` word yields
            // just itself.
            for frag in self.cmd_fragments(arg_tok, config) {
                collect_substitution_heads(
                    &sm,
                    self.registry,
                    frag,
                    config,
                    &mut heads,
                    &mut expr_toks,
                );
            }
            (heads, expr_toks)
        };
        self.push_collected_heads(heads, scope_path);
        // Math-function applications inside any nested `[expr {…}]` dispatch
        // to `::tcl::mathfunc::<fn>` — recorded here (with `&mut self`) because
        // the free-function collection can't resolve that namespace.
        for expr_tok in expr_toks {
            self.record_expr_function_invocations(expr_tok, scope_path);
        }
    }

    /// Recognise the tcllib `namespace eval $ns [list namespace unknown
    /// $handler]` idiom (issue #923 idx 110): the ``[...]`` body is a
    /// `Cmd` token, so [`Self::analyse_body`]'s literal-`{...}`-only gate
    /// never walks it as a script, and the generic nested-substitution
    /// scan resolves the segment's head to `list` (never dispatching
    /// `AnalyserHookId::NamespaceUnknown`) — so the handler installation
    /// is invisible to every existing path. Narrowly recognises the
    /// exact `list namespace unknown ?HANDLER?` shape and, on a match,
    /// calls [`Self::handle_namespace_unknown_command`] unmodified with
    /// a synthesised `["unknown", HANDLER?]` args slice, reusing its
    /// established empty/query-form gating rather than reimplementing
    /// it.
    ///
    /// Deliberately narrow: does not recognise the same idiom built via
    /// `concat`, `format`, `linsert`, string concatenation, or a
    /// `list`-building helper proc — a documented scope boundary, not
    /// an oversight (no attested real-world instance of those forms).
    pub(super) fn detect_list_wrapped_namespace_unknown(&mut self, body_tok: Token) {
        if body_tok.kind != TokenType::Cmd {
            return;
        }
        let config = self.lexer_config();
        // Collect the descended segments first (this borrows
        // `self.source` through the `SourceMap`); call the `&mut self`
        // handler afterwards, once the immutable borrow has ended — the
        // same two-phase shape as `record_invocations_from_cmd_token`.
        let segs: Vec<SegmentedCommand> = {
            let sm = SourceMap::new(&self.source);
            let mut segs = Vec::new();
            for frag in self.cmd_fragments(body_tok, config) {
                if frag.kind != TokenType::Cmd || sm.token_text(frag).is_empty() {
                    continue;
                }
                let descended = descend_token(&sm, frag, config);
                segs.extend(segments_from_tree(descended.tree(), &sm));
            }
            segs
        };
        for seg in &segs {
            if seg.texts.len() < 3
                || seg.texts.len() > 4
                || seg.texts[0] != "list"
                || seg.texts[1] != "namespace"
                || seg.texts[2] != "unknown"
            {
                continue;
            }
            self.handle_namespace_unknown_command(&seg.texts[2..]);
        }
    }

    /// Run the per-command syntactic dispatch
    /// ([`Self::emit_dispatch_site_diagnostics`] — security W101-W312,
    /// bounds W230-W242, W001 / W004 / W304, arity E002-E003, …) on every
    /// command nested in a ``[…]`` substitution of this command's words,
    /// recursing into further nested substitutions and the nested
    /// commands' own bodies.  The main analyser walk descends proc /
    /// control-flow *bodies* (so those commands are already checked) but
    /// never a ``[…]`` substitution, which it treats as an opaque value —
    /// so `set fh [open "|$cmd" r]` / `set x [string index abc 99]` would
    /// otherwise escape the per-command checks.
    ///
    /// Only ``[…]`` regions are entered here; everything reached from
    /// inside one is invisible to the main walk, so the recursion may
    /// freely descend the nested commands' own bodies and substitutions
    /// without double-firing a diagnostic the main walk already emitted.
    /// `scope_path` is the enclosing command's scope — a substitution
    /// runs in the same frame as the command it is embedded in.
    fn run_nested_command_diagnostics(&mut self, arg_tokens_in: &[Token], scope_path: &[usize]) {
        // Collect the descended substitution commands first (this borrows
        // `self.source` through the `SourceMap`); run the `&mut self`
        // dispatch afterwards, once the immutable borrow has ended.  Each
        // `SegmentedCommand` is fully owned (absolute spans), so it
        // outlives the borrow.
        let config = self.lexer_config();
        let mut nested: Vec<SegmentedCommand> = Vec::new();
        {
            let sm = Analyser::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            );
            for arg_tok in arg_tokens_in {
                let start = arg_tok.span.start() as usize;
                let end = arg_tok.span.end() as usize;
                if start > self.source.len() || end > self.source.len() || start > end {
                    continue;
                }
                match arg_tok.kind {
                    TokenType::Cmd => {
                        for frag in self.cmd_fragments(*arg_tok, config) {
                            collect_substitution_segments(
                                &sm,
                                self.registry,
                                frag,
                                config,
                                &mut nested,
                            );
                        }
                    }
                    // A quoted / bareword / compound word (`Esc`) may carry
                    // live `[...]` substitutions (`log "got [HTTP::uri]"`).
                    // The bare-`Cmd` walk never enters it, so its substitution
                    // commands escape every per-command check (IRULE3102, W123,
                    // …).  A braced `Str` data word's `[...]` is literal and is
                    // skipped; braced *expr* args are covered by
                    // `run_nested_expr_diagnostics`.
                    TokenType::Esc => {
                        let arg_src = &self.source[start..end];
                        if !arg_src.contains('[') {
                            continue;
                        }
                        for (off, inner) in top_level_cmd_subst_regions(arg_src) {
                            let off = u32::try_from(off)
                                .expect("byte offset fits in u32 for in-memory source");
                            let base = arg_tok.span.start() + off;
                            for seg in crate::segmenter::segment_commands_with_offset_and_config(
                                inner, base, config,
                            ) {
                                collect_segment_recursive(
                                    &sm,
                                    self.registry,
                                    seg,
                                    config,
                                    &mut nested,
                                    0,
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        for seg in nested {
            self.dispatch_nested_segment(&seg, scope_path);
        }
    }

    /// Run the per-command syntactic dispatch on every command nested in a
    /// ``[…]`` substitution that appears inside a *braced expression*
    /// argument (`if { [acl_ok] } …`, `while { [done $x] } …`).  Such an
    /// argument is a `Str` (braced) token, so it is opaque to
    /// [`Self::run_nested_command_diagnostics`] (which only descends bare
    /// `Cmd` argument tokens) — yet the expression's `[…]` substitutions
    /// are live commands the main walk never reaches.
    fn run_nested_expr_diagnostics(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        let expr_indices: Vec<usize> = match self.registry {
            Some(r) => {
                let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
                r.arg_indices_for_role(cmd_name, &arg_strs, ArgRole::Expr)
            }
            None => return,
        };
        if expr_indices.is_empty() {
            return;
        }
        let config = self.lexer_config();
        let mut nested: Vec<SegmentedCommand> = Vec::new();
        {
            let sm = Analyser::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            );
            for idx in expr_indices {
                // Only a *braced* expr arg is opaque to the bare-`Cmd` walk;
                // an unbraced `[…]` expr arg is itself a `Cmd` token already
                // descended by `run_nested_command_diagnostics`.
                let Some(tok) = arg_tokens.get(idx) else {
                    continue;
                };
                if tok.kind != TokenType::Str {
                    continue;
                }
                // Re-lex the braced expression as a script: its operands
                // (`$x`, `+`, literals) are not commands, but each `[…]`
                // substitution still tokenises as a `Cmd` to descend.
                let descended = descend_token(&sm, *tok, config);
                for seg in segments_from_tree(descended.tree(), &sm) {
                    for inner in &seg.all_tokens {
                        if inner.kind == TokenType::Cmd {
                            collect_substitution_segments(
                                &sm,
                                self.registry,
                                *inner,
                                config,
                                &mut nested,
                            );
                        }
                    }
                }
            }
        }
        for seg in nested {
            self.dispatch_nested_segment(&seg, scope_path);
        }
    }

    /// Run the full per-command diagnostic dispatch on one command
    /// descended from a ``[…]`` substitution: the syntactic emitters
    /// ([`Self::emit_dispatch_site_diagnostics`]) plus the EXPR-argument
    /// walk ([`Self::dispatch_expr_arguments`], which hosts W100 / W110 /
    /// W114 / W003).  The main walk runs both on a top-level command but
    /// only the syntactic half reaches substitution commands by
    /// default, so an unbraced `expr` inside `[…]`
    /// (`set y [expr $a + $b]`) would otherwise escape W100.
    fn dispatch_nested_segment(&mut self, seg: &SegmentedCommand, scope_path: &[usize]) {
        if seg.texts.is_empty() || seg.argv.is_empty() {
            return;
        }
        let cmd_name = seg.texts[0].clone();
        let cmd_tok = seg.argv[0];
        // Safe-interpreter visibility gate (issue #1001): a `[…]` bracket
        // substitution always invokes its head immediately, wherever it
        // appears — `set x [source b.tcl]`, `if {[exec ls] ne ""} …` — so a
        // command nested this way must pass the same gate a top-level
        // command does (`Self::safe_interp_visibility_gate`'s doc). Every
        // nesting depth this walker's caller
        // (`collect_substitution_segments` / `collect_segment_recursive`)
        // already discovers is covered for free; a no-op outside a tracked
        // safe interpreter.
        if self.safe_interp_visibility_gate(&cmd_name, cmd_tok) {
            return;
        }
        let args = seg.texts.get(1..).unwrap_or(&[]);
        let arg_tokens = seg.argv.get(1..).unwrap_or(&[]);
        // A tracked `namespace ensemble ... -map` redirect to a hidden
        // command (issue #1001 follow-up): `cmd_name` isn't itself a hidden
        // registry name, but its resolved dispatch target might be — see
        // `check_ensemble_redirect_hiding`'s doc.
        if self.check_ensemble_redirect_hiding(&cmd_name, args, arg_tokens, scope_path) {
            return;
        }
        let arg_single = seg.single_token_word.get(1..).unwrap_or(&[]);
        // `emit_arity_diagnostics` expects the expand array parallel to
        // the *full* argv (head at index 0), matching `process_command`.
        let arg_expand = seg.expand_word.as_deref().unwrap_or(&[]);
        self.emit_dispatch_site_diagnostics(&DispatchSite {
            cmd_name: &cmd_name,
            args,
            arg_tokens,
            arg_single,
            arg_expand_in: arg_expand,
            cmd_tok,
            scope_path,
        });
        self.dispatch_expr_arguments(&cmd_name, args, arg_tokens, cmd_tok);
        // W216 (broken brace-form array access, `${arr}(idx)` / `${arr($i)}`)
        // must reach substitution commands too: `set v [puts ${arr}(name)]`
        // hides the offending word inside a `[…]`, which the main `walk_body`
        // pass treats as an opaque value.  Without this the nested word escapes
        // the check entirely, so the brace-then-paren emitter must run on
        // substitution commands too.
        self.emit_w216_brace_then_paren(seg);
        // A `VarWrite`-role command nested in a `[…]` substitution still
        // writes its variable arguments into the enclosing scope — the
        // idiomatic `if {[regexp {…} $s m]} {…}` / `set n [scan $s "%d" x]` /
        // `while {[gets $chan line] >= 0} {…}` — so bind them for
        // completion/hover/definition and read-before-set, just as the
        // top-level `process_command` path does.
        self.handle_var_binding_command(&cmd_name, args, arg_tokens, scope_path);
        // Record the nested command's variable-/command-substitution-as-command
        // call site too (`puts [$obj method]`, `if {[$obj ok]} …`).  The main
        // walk treats `[…]` as a value, so without this the W307 multi-dispatch
        // suppression under-counts `$obj` dispatches that live inside command
        // substitutions and the W307/W308 emitters never see them.
        self.record_var_or_cmd_command_site(
            cmd_tok,
            arg_expand.first().copied().unwrap_or(false),
            args,
            arg_tokens,
            arg_expand.get(1..).unwrap_or(&[]),
            scope_path,
        );
        // W125 (orphaned keyword) and IRULE5005 (direct iRules-proc call
        // without `call`) key off whether the head resolves to a user proc, so
        // they must reach substitution commands too: `when HTTP_REQUEST { set x
        // [helper] }` invokes `helper` directly inside a `[…]`, and without this
        // the IRULE5005 emitter never sees it.  Matches the top-level
        // `process_command` ordering (proc-resolution after site recording).
        self.emit_proc_resolution_diagnostics(&cmd_name, args, cmd_tok, scope_path);
        // A `package require` nested in a `[…]` substitution still runs — the
        // guarded-optional-dependency idiom puts it exactly there
        // (`if {[catch {package require Tk} err]} { … fallback … }`), and the
        // body collector descends `catch`'s script argument to reach it. Without
        // this, W120 ("requires `package require Tk`") false-positives on every
        // file using the standard guard, and the W123 conservative
        // any-require-seen gate never engages.  Only the two `package`
        // hooks run here — the substitution path deliberately dispatches
        // no other handler family.
        match self.resolve_analyser_hook(&cmd_name, args) {
            Some(tcl_registry::hooks::AnalyserHookId::PackageRequire) => {
                self.handle_package_require(cmd_tok, args, arg_tokens);
            }
            Some(tcl_registry::hooks::AnalyserHookId::PackageProvide) => {
                self.handle_package_provide(cmd_tok, args);
            }
            _ => {}
        }
        // A variable-binding tail (`catch SCRIPT ?resultVar? ?optionsVar?`)
        // nested in a `[...]` substitution (`set out [catch {…} msg]`,
        // `if {[catch {…} e]} …`) still binds its variables in the enclosing
        // scope, so record them for `symbols`/completion/hover — var-defs are
        // collected from substitution commands too.  The bound positions come
        // from the registry's `ArgRole::VarWrite` rows, not a hardcoded
        // `catch` shape.
        // `warn_if_unused = false`: the binding is a command side effect,
        // not a "set but never used" target (no W211).
        if cmd_name == "catch"
            && let Some(registry) = self.registry
        {
            let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
            for i in registry.arg_indices_for_role(&cmd_name, &arg_strs, ArgRole::VarWrite) {
                if let (Some(name), Some(tok)) = (args.get(i), arg_tokens.get(i)) {
                    let name = name.clone();
                    self.define_var(&name, *tok, scope_path, false, None);
                }
            }
        }

        // A definition command (`proc`, a class definer, `oo::define`)
        // nested inside a substitution — the feature-detection idiom
        // `if {![catch {oo::configurable create Greeter {…}}]} {…}` — still
        // defines its procedure/class and walks its body by the definer
        // grammar, exactly as the top-level dispatch does.  Without this the
        // definer's member keywords (`property`, `constructor`) and the
        // defined name (`Greeter`) all draw W123 as unknown commands, even
        // though W002 already reported the dialect-gated definer once.  The
        // generic collector skips these bodies (`definition_handler_owns_body`)
        // so members are never also dispatched as plain commands.  The
        // dispatch mirrors the top-level chain exactly: the stamped `Proc` /
        // `OoDefine` hooks first (the handlers no longer name-guard
        // themselves), then the grammar-driven definer trio only on the
        // hookless path.
        // Each handler returns whether it claimed the command; nothing
        // follows this dispatch, so an unclaimed command simply ends the
        // substitution walk the same way a claimed one does.
        {
            use tcl_registry::hooks::AnalyserHookId as Hook;
            match self.resolve_analyser_hook(&cmd_name, args) {
                Some(Hook::Proc) => {
                    self.handle_proc_command(args, arg_tokens, arg_single, scope_path);
                }
                Some(Hook::OptProc) => {
                    self.handle_opt_proc_command(args, arg_tokens, arg_single, scope_path);
                }
                Some(Hook::OoDefine) => {
                    self.handle_oo_define_command(&cmd_name, args, arg_tokens, scope_path);
                }
                None => {
                    let _claimed = self
                        .handle_oo_class_command(&cmd_name, args, arg_tokens, scope_path)
                        || self.handle_snit_type_command(&cmd_name, args, arg_tokens, scope_path)
                        || self.handle_itcl_class_command(&cmd_name, args, arg_tokens, scope_path);
                }
                Some(_) => {}
            }
        }
    }

    /// The `[…]` substitution fragment tokens of a (possibly compound)
    /// `Cmd`-headed word, with absolute spans.  Re-lexing the word slice
    /// recovers the per-fragment boundaries the argv merge erased, so
    /// `[foo]bar` yields its `[foo]` fragment (not the whole word) and
    /// `[foo]bar[baz]` yields both `[foo]` and `[baz]`.  On a lex error or
    /// a degenerate empty/out-of-bounds span, falls back to the token as
    /// given so the caller still descends *something*.
    fn cmd_fragments(&self, arg_tok: Token, config: LexerConfig) -> Vec<Token> {
        let start = arg_tok.span.start() as usize;
        let end = arg_tok.span.end() as usize;
        if start >= self.source.len() || end > self.source.len() || start >= end {
            return vec![arg_tok];
        }
        let base = arg_tok.span.start();
        let frags: Vec<Token> =
            Lexer::with_source_map(SourceMap::new(&self.source[start..end]), config)
                .tokenise_all()
                .map(|toks| {
                    toks.into_iter()
                        .filter(|t| t.kind == TokenType::Cmd)
                        .map(|t| Token {
                            kind: t.kind,
                            span: Span::new(t.span.start() + base, t.span.end() + base),
                            content_offset: t.content_offset,
                            in_quote: t.in_quote,
                        })
                        .collect()
                })
                .unwrap_or_default();
        if frags.is_empty() {
            vec![arg_tok]
        } else {
            frags
        }
    }

    /// Inner: a braced ``Str`` *expression* argument (`if {[acl_ok]} …`).
    /// Record the command substitutions inside the expression — the
    /// expression's own operands are not commands (see
    /// [`collect_expr_substitutions`]).  Skips the over-recording the
    /// generic word scanner would do on a braced *data* word.
    fn record_invocations_from_expr_token(&mut self, expr_tok: Token, scope_path: &[usize]) {
        let config = self.lexer_config();
        let (heads, expr_toks) = {
            let sm = Analyser::source_map(
                &self.source,
                &self.cached_line_index,
                self.cached_line_index_source_len,
            );
            let mut heads: Vec<CollectedHead> = Vec::new();
            let mut expr_toks: Vec<Token> = Vec::new();
            collect_expr_substitutions(
                &sm,
                self.registry,
                expr_tok,
                config,
                &mut heads,
                &mut expr_toks,
            );
            (heads, expr_toks)
        };
        self.push_collected_heads(heads, scope_path);
        // This expr's own math functions, plus any nested inside a `[expr {…}]`
        // substitution the collection surfaced (`if {[expr {Pi()}]}`).
        self.record_expr_function_invocations(expr_tok, scope_path);
        for nested in expr_toks {
            self.record_expr_function_invocations(nested, scope_path);
        }
    }

    /// Every math-function application inside the expression `expr_tok`, as
    /// `(name, name_span, arg_count)` with the function-name span mapped to
    /// absolute source coordinates.  The single extraction the invocation
    /// recorder and the dialect-availability diagnostic both read.
    ///
    /// The expression body starts past the opening delimiter; the AST's
    /// offsets are relative to the *trimmed* text, so each is translated back
    /// through the leading-whitespace trim to a source span.
    pub(in crate::analyser) fn expr_function_calls(
        &self,
        expr_tok: Token,
    ) -> Vec<(String, Span, usize)> {
        let content_start = expr_tok.span.start() + u32::from(expr_tok.content_offset);
        let (start, end) = (content_start as usize, expr_tok.span.end() as usize);
        if start > end || end > self.source.len() {
            return Vec::new();
        }
        let expr_text = &self.source[start..end];
        let trimmed = expr_text.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }
        let trim_base = u32::try_from(expr_text.len() - expr_text.trim_start().len()).unwrap_or(0);
        let parsed = crate::parse_expr(trimmed, Some(self.dialect()));
        parsed
            .function_calls()
            .into_iter()
            .map(|(name, rel_start, argc)| {
                let name_start = content_start + trim_base + rel_start;
                let span = Span::new(
                    name_start,
                    name_start + u32::try_from(name.len()).unwrap_or(0),
                );
                (name.to_owned(), span, argc)
            })
            .collect()
    }

    /// Record each math-function application (`sin($x)`, `max($a, $b)`) as an
    /// invocation of the command it dispatches to, `::tcl::mathfunc::<name>`.
    /// A user who defines `proc ::tcl::mathfunc::myfunc { … }` — or, per TIP
    /// 232, a namespace-local `proc ::ns::tcl::mathfunc::myfunc` that shadows
    /// it inside `::ns` — then gets go-to-definition, references, rename, and
    /// arity checking on `myfunc(...)` calls, and the function is no longer
    /// reported unused.
    ///
    /// The written head is the bare function word (`sin`); the resolved name
    /// is the *local-first* candidate `{ns}::tcl::mathfunc::sin` for the
    /// call's own namespace, computed the same way an ordinary bareword
    /// command's walk-time guess is ([`Self::resolve_command_qualified_name`])
    /// so a rename rewrites only the tail token in the expression.
    /// [`Self::finalise_invocation_resolutions`] settles it against the real
    /// two-candidate rule (the caller's `tcl::mathfunc`, else the global
    /// one) — never the generic one-hop `{ns}::{name}` suffix-strip it uses
    /// for an ordinary call, which would misparse the fixed `tcl::mathfunc`
    /// dispatch segment as if it were the calling namespace and could
    /// mis-resolve to an unrelated global command sharing the bare tail name.
    fn record_expr_function_invocations(&mut self, expr_tok: Token, scope_path: &[usize]) {
        for (name, span, argc) in self.expr_function_calls(expr_tok) {
            let resolved =
                self.resolve_command_qualified_name(&format!("tcl::mathfunc::{name}"), scope_path);
            self.push_mathfunc_command_reference(name, span, resolved, Some(argc));
        }
    }

    /// Resolve each collected `(name, span, argc)` head to a qualified name and
    /// push it as a `command_invocations` entry.  `argc` carries the nested call's
    /// statically-known argument count (`None` when `{*}`-expanded), so a wrong-arg
    /// call to a cross-file proc *inside a substitution* (`set x [helper a b c]`)
    /// still draws the cross-file arity error.
    fn push_collected_heads(&mut self, heads: Vec<CollectedHead>, scope_path: &[usize]) {
        for (name, range, argc, callback_arity, sub_candidate) in heads {
            let resolved = self.resolve_command_qualified_name(&name, scope_path);
            // `[<ensemble> <subcommand> …]` nested inside a substitution —
            // the same existence-probed subcommand reference a top-level
            // `<ensemble> <subcommand> …` call already gets from
            // `record_ensemble_subcommand_invocation` (issue #923 idx 106).
            // `argc` here is "args after the head" (the subcommand word
            // included), so it shifts by one to become "args after the
            // subcommand word" — the same convention that function uses.
            if let Some((sub, sub_span)) = sub_candidate
                && let Some(target) = self
                    .result
                    .ensemble_subcommand_targets
                    .get(&resolved)
                    .and_then(|subs| subs.get(&sub))
                    .cloned()
            {
                let sub_argc = argc.map(|a| a.saturating_sub(1));
                self.push_command_reference_with_policy(sub, sub_span, target, sub_argc, true);
            }
            self.result.command_invocations.push(
                crate::signature_scan::types::SignatureCommandInvocation {
                    name,
                    range,
                    resolved_qualified_name: Some(resolved),
                    resolution_candidates: Vec::new(),
                    argc,
                    callback_arity,
                    callback_baked_args: 0,
                    indirect: false,
                    rename_safe: true,
                    existence_probe: false,
                    is_mathfunc_call: false,
                },
            );
        }
    }

    /// Inner: ``Esc`` (bareword / quoted) and ``Str`` (braced)
    /// tokens.  For ``Str``, strip the surrounding ``{…}`` via
    /// ``content_offset`` first — otherwise the scanner sees the
    /// outer ``{`` and skips the entire braced region opaquely,
    /// missing every nested ``[cmd]`` inside braced expr args.
    fn record_invocations_from_word_token(
        &mut self,
        arg_tok: Token,
        arg_src: &str,
        arg_start: u32,
        scope_path: &[usize],
    ) {
        let (inner_src, inner_base) = if matches!(arg_tok.kind, TokenType::Str) {
            let inner_off = arg_tok.content_offset as usize;
            if inner_off <= arg_src.len() {
                let trimmed = arg_src[inner_off..]
                    .strip_suffix('}')
                    .unwrap_or(&arg_src[inner_off..]);
                let inner_off_u32 = u32::try_from(inner_off)
                    .expect("content_offset fits in u32 for in-memory source");
                (trimmed, arg_start + inner_off_u32)
            } else {
                (arg_src, arg_start)
            }
        } else {
            (arg_src, arg_start)
        };
        for (name, off) in scan_nested_command_heads(inner_src) {
            let abs_start = inner_base + off;
            let abs_end = abs_start
                + u32::try_from(name.len()).expect("token length fits in u32 for in-memory source");
            let resolved = self.resolve_command_qualified_name(&name, scope_path);
            self.result.command_invocations.push(
                crate::signature_scan::types::SignatureCommandInvocation {
                    name,
                    range: tcl_lexer::Span::new(abs_start, abs_end),
                    resolved_qualified_name: Some(resolved),
                    resolution_candidates: Vec::new(),
                    // Nested `[cmd ...]` head, no recorded argument list — arity skip.
                    argc: None,
                    callback_arity: None,
                    callback_baked_args: 0,
                    indirect: false,
                    rename_safe: true,
                    existence_probe: false,
                    is_mathfunc_call: false,
                },
            );
        }
    }

    /// Record this command as a variable-as-command (``$obj
    /// method ...``) or command-substitution-as-command
    /// (``[expr ...] args``) call site so the post-walk W307 /
    /// W308 emitters can resolve them.
    fn record_var_or_cmd_command_site(
        &mut self,
        cmd_tok: Token,
        head_expanded: bool,
        args: &[String],
        arg_tokens: &[Token],
        arg_expand: &[bool],
        scope_path: &[usize],
    ) {
        let in_method = self.scope_path_in_method_body(scope_path);
        // Content span of the method word (delimiters trimmed) — the tight
        // W308 anchor and "did you mean" fix target.
        let method_span = arg_tokens.first().map(|t| {
            tcl_lexer::Span::new(t.span.start() + u32::from(t.content_offset), t.span.end())
        });
        match cmd_tok.kind {
            TokenType::Var => {
                let sm = Analyser::source_map(
                    &self.source,
                    &self.cached_line_index,
                    self.cached_line_index_source_len,
                );
                // A composite head whose first token is a *braced* variable
                // (`${ns}::define::[…]`) merges into one Var word token, so the
                // raw text spans the whole word.  The dispatched variable is
                // only the braced name (`${ns}` → `ns`); the `}` closes it and
                // the rest is a literal / substituted suffix.  Read the
                // first VAR sub-token's clean name by truncating
                // at the first `}` (a simple `$obj` or namespaced `$ns::v` head
                // contains no `}` and is unchanged).
                let raw = sm.token_text(cmd_tok);
                let var_name = raw
                    .split_once('}')
                    .map_or(raw, |(name, _)| name)
                    .to_string();
                let method_name = args.first().cloned();
                // M7: a simple-`$cmd` head may be a statically-known
                // dispatch.  Record the *site* for settlement in the
                // CFG/SSA phase, where the flow-sensitive value model can
                // prove (or soundly refuse to prove) the finite set of
                // command names reaching this exact program point —
                // never the walk's lexical constant map, whose
                // last-write-wins view collapses `if`/loop joins (issue
                // #945 fault 2).  A braced composite head (`${ns}::tail
                // …`) is the W307 ensemble shape, not a whole-command
                // variable, so it is skipped.
                if var_name == raw {
                    let ns = self.command_resolution_namespace(scope_path);
                    self.pending_const_dispatches
                        .push(super::state::ConstDispatchSite {
                            var_name: var_name.clone(),
                            span: cmd_tok.span,
                            ns,
                            head_expanded,
                        });
                }
                self.var_command_sites.push(super::state::VarCommandSite {
                    var_name,
                    method_name,
                    method_span,
                    cmd_span: cmd_tok.span,
                    in_method,
                    argc: args.len(),
                    has_expand: arg_expand.iter().any(|&e| e),
                });
            }
            TokenType::Cmd => {
                let sm = Analyser::source_map(
                    &self.source,
                    &self.cached_line_index,
                    self.cached_line_index_source_len,
                );
                let cmd_text = sm.token_text(cmd_tok).to_string();
                let method_name = args.first().cloned();
                self.cmd_command_sites.push(super::state::CmdCommandSite {
                    cmd_text,
                    method_name,
                    method_span,
                    cmd_span: cmd_tok.span,
                    in_method,
                });
            }
            _ => {}
        }
    }

    /// Detect `TclOO` instance creation and record the resulting
    /// variable / instance-command → class mapping in
    /// [`AnalysisResult::instance_classes`].  Three patterns:
    ///
    /// * `set VAR [CLASS new ?args?]`
    /// * `set VAR [CLASS create NAME ?args?]`
    /// * `CLASS create VAR ?args?`
    ///
    /// `CLASS` must resolve to a user-defined class in
    /// `result.all_classes` (so `oo::class create Dog` — which
    /// defines a *class*, not an instance — is naturally
    /// excluded because `oo::class` isn't a user class).
    /// Best-effort and not flow-sensitive: the last assignment
    /// to a given name wins.
    pub(crate) fn record_instance_creation(&mut self, cmd_name: &str, args: &[String]) {
        // Registry `defines_command_at` — a spec-declared argument whose
        // literal value becomes a callable command name (`coroutine NAME cmd
        // …`, `interp create NAME`).  Registry data only, so it is recorded
        // eagerly (like the factory binding below) and works inside an
        // isolated proc body too.
        self.record_registry_defined_command(cmd_name, args);
        // Registry object-factories (`struct::graph g` naming form, `set g
        // [struct::graph …]` return form) resolve from the registry alone — no
        // `all_classes` — so bind them IMMEDIATELY, even inside an isolated proc
        // body.  Deferring these (as the per-item firewall does for user-class
        // instances) would leave `instance_classes` empty during that body's own
        // `$g walk … -command cb` recording, dropping the callback edge /
        // reference for an in-proc instance-method callback and diverging the
        // incremental path from the whole-file walk.
        let bound_registry_factory = self.record_registry_factory_instance(cmd_name, args);

        // Per-item isolated proc body: a *user*-class instance can't be resolved
        // here (`all_classes` is empty), so capture the raw `(command, args)` for
        // the two instance-creation shapes and let the graft replay them against
        // the shell's full `all_classes` instead (see `pending_instances`).
        if let Some(pending) = self.pending_instances.as_mut() {
            let shape_a =
                cmd_name == "set" && args.len() >= 2 && args[1].trim_start().starts_with('[');
            let shape_b = args.len() >= 2 && args[0] == "create";
            // A registry factory already bound above needs no user-class replay.
            if (shape_a || shape_b) && !bound_registry_factory {
                pending.push((cmd_name.to_owned(), args.to_vec()));
            }
            return;
        }
        if bound_registry_factory {
            return;
        }
        // Pattern A: `set VAR [CLASS new|create ...]` — a *user* class (the
        // registry-factory subset is handled by `record_registry_factory_instance`
        // above).
        if cmd_name == "set"
            && args.len() >= 2
            && let Some(class_q) = self.class_from_constructor_subst(&args[1])
        {
            self.result
                .instance_classes
                .insert(args[0].clone(), class_q);
            return;
        }
        // Pattern B: `CLASS create NAME ...` — `NAME` (argv[1]) names a new
        // instance command.
        if args.len() >= 2 && args[0] == "create" && is_plain_created_name(&args[1]) {
            let name = &args[1];
            if let Some(class_q) = self.resolve_user_class(cmd_name) {
                // Known user class: record the object → class mapping (for
                // `$obj method` / method validation) and the created command
                // name.
                self.result.instance_classes.insert(name.clone(), class_q);
                self.result.created_instance_commands.insert(name.clone());
            } else if self.command_head_could_be_external_class(cmd_name) {
                // Unknown (external-package) class: the `create NAME` idiom
                // still binds a new command, so register the name to suppress
                // the spurious W123 / W307 on later `NAME method` dispatch
                // (issue #777).  The class identity is unknown, so no
                // `instance_classes` entry (that would enable W308 method
                // validation we can't perform).
                self.result.created_instance_commands.insert(name.clone());
            }
        }
    }

    /// Bind an object handle created by a *registry* object-factory — one
    /// resolvable from the registry alone (no `all_classes`), so it is recorded
    /// eagerly even inside an isolated proc body (unlike a user-class instance,
    /// which must defer to the graft).  Two shapes:
    ///
    /// * naming factory  `struct::graph g`         → `g` (via `creates_instance_at`)
    /// * factory return  `set g [struct::graph …]` → `g`
    ///
    /// Returns whether a binding was recorded.
    fn record_registry_factory_instance(&mut self, cmd_name: &str, args: &[String]) -> bool {
        // Naming factory: a command whose spec declares `creates_instance_at`
        // binds the object command it names positionally (`report::report
        // reportName …`, `struct::graph g`) — the naming shape is registry data,
        // no hardcoded `create` / `new` idiom.
        let factory = self
            .registry
            .as_ref()
            .and_then(|r| r.get(cmd_name))
            .and_then(|s| {
                s.creates_instance_at
                    .map(|idx| (idx, s.object_class.map(|oc| oc.class_name.to_string())))
            });
        if let Some((idx, class_name)) = factory
            && let Some(name) = args.get(idx as usize)
            && is_plain_created_name(name)
            // A `?name?` slot can instead hold a deserialise *operator*
            // (`struct::graph = $serial` / `:= ` / `as ` / `deserialize `), which
            // names no object command — never bind the operator token itself.
            && !matches!(name.as_str(), "=" | ":=" | "as" | "deserialize")
        {
            let class = class_name.unwrap_or_else(|| cmd_name.to_string());
            self.bind_registry_instance_class(name.clone(), class);
            self.result.created_instance_commands.insert(name.clone());
            return true;
        }
        // Factory-return: `set g [struct::graph …]` — the registry-factory subset
        // of `class_from_constructor_subst`.  A user-class `[Class new]` returns
        // `None` here and is left to Pattern A / the graft (it needs `all_classes`).
        if cmd_name == "set"
            && args.len() >= 2
            && let Some(class) = self.registry_factory_class_from_subst(&args[1])
        {
            self.bind_registry_instance_class(args[0].clone(), class);
            return true;
        }
        false
    }

    /// Record a command name bound by a registry `defines_command_at` spec —
    /// the argument whose *literal* value becomes a callable command once the
    /// call runs (`coroutine NAME cmd ?arg …?` at the command level, `interp
    /// create ?-safe? ?--? ?NAME?` at the subcommand level).  The name goes
    /// into [`AnalysisResult::created_instance_commands`] so the W123
    /// unresolved-command pass treats later calls to it as resolved.  Purely
    /// registry-driven — no command name is matched here: `idx` is shifted
    /// past any leading declared option words
    /// ([`tcl_registry::CommandSpec::leading_option_word_count`] /
    /// [`tcl_registry::SubCommand::leading_option_word_count`], the same
    /// declared-option table `interp create`'s own doc comment already names)
    /// so `interp create -safe NAME` lands `idx` on `NAME`, not `-safe`.
    ///
    /// Conservative by construction: only a plain literal word is recorded (a
    /// `$var` / `[cmd]` / `%AUTO%` name is runtime-computed), and a word that
    /// still starts with `-` after the option-word skip is an undeclared or
    /// ambiguous flag, never a name — a missing name is auto-generated at run
    /// time and needs no recording.
    fn record_registry_defined_command(&mut self, cmd_name: &str, args: &[String]) {
        let Some(reg) = self.registry else {
            return;
        };
        let Some(spec) = reg.get(cmd_name) else {
            return;
        };
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        // Command-level index, else the resolved subcommand's index shifted
        // past the subcommand word itself — either way, further shifted past
        // any leading declared option words the command/subcommand accepts
        // before its name argument.
        let name_idx = if let Some(idx) = spec.defines_command_at {
            Some(usize::from(idx) + spec.leading_option_word_count(&arg_strs))
        } else {
            args.first()
                .and_then(|sub| spec.resolve_subcommand(sub))
                .and_then(|sub| {
                    let idx = usize::from(sub.defines_command_at?) + 1;
                    Some(idx + sub.leading_option_word_count(arg_strs.get(1..).unwrap_or(&[])))
                })
        };
        let Some(idx) = name_idx else {
            return;
        };
        let Some(name) = args.get(idx) else {
            return;
        };
        // A word right after a consumed `--` terminator is positional no
        // matter its shape (Tcl's own convention: `--` means "every later
        // word is an argument, not an option"), so `interp create -- -safe`
        // must record `-safe` as the created command rather than treating it
        // like the undeclared/ambiguous flag it would be without that `--`.
        let past_double_dash = idx
            .checked_sub(1)
            .and_then(|i| arg_strs.get(i))
            .is_some_and(|&w| w == "--");
        if (!past_double_dash && name.starts_with('-')) || !is_plain_created_name(name) {
            return;
        }
        self.result.created_instance_commands.insert(name.clone());
    }

    /// Collision-safe `instance_classes` insertion for the two *registry*
    /// object-factory binding sites above (Tk widget paths, tcllib naming
    /// factories) — unlike `instance_classes`' general last-write-wins
    /// contract, a name seen bound to two *different* registry classes
    /// anywhere in the file is dropped and never re-added, so a consumer
    /// that needs soundness (`widget_command.rs`'s W001/E002/E003 — issue
    /// #927) can trust a present entry unconditionally. Scoped to these two
    /// call sites only: the `TclOO` user-class paths in
    /// `record_instance_creation` keep their existing documented
    /// best-effort behaviour unchanged.
    fn bind_registry_instance_class(&mut self, name: String, class: String) {
        if self.result.ambiguous_instance_names.contains(&name) {
            return;
        }
        match self.result.instance_classes.get(&name) {
            Some(existing) if *existing != class => {
                self.result.instance_classes.remove(&name);
                self.result.ambiguous_instance_names.insert(name);
            }
            _ => {
                self.result.instance_classes.insert(name, class);
            }
        }
    }

    /// The class named by a `[factory …]` command-substitution when `factory` is
    /// a registry object-factory (`struct::graph` / `struct::tree` — a
    /// `creates_instance_at` + `object_class` command whose result is an instance
    /// of its own class).  `None` for a user-class `[Class new]` (which needs
    /// `all_classes`, resolved by `class_from_constructor_subst`).
    fn registry_factory_class_from_subst(&self, value: &str) -> Option<String> {
        let inner = value.trim().strip_prefix('[')?.strip_suffix(']')?;
        let head = inner.split_whitespace().next()?;
        let reg = self.registry?;
        if reg
            .get(head)
            .is_some_and(|s| s.creates_instance_at.is_some())
        {
            return reg.object_class(head).map(|c| c.class_name.to_string());
        }
        None
    }

    /// Whether `head` is a plausible external-package class command in a
    /// `head create NAME` construct: a plain bareword (not a computed
    /// `$…`/`[…]` head) that the analyser can't otherwise resolve — not a
    /// registry builtin (so `dict create`, `interp create`, the `oo::*`
    /// metaclasses, … are excluded), and not a user proc.  Under those
    /// conditions the `create NAME` idiom is almost certainly object
    /// construction whose created command name should be recognised.
    fn command_head_could_be_external_class(&self, head: &str) -> bool {
        if head.is_empty() || head.starts_with(['$', '[']) {
            return false;
        }
        let known = self
            .registry
            .as_ref()
            .is_some_and(|r| r.get(head).is_some())
            || self.result.all_procs.contains_key(head)
            || self.result.all_procs.contains_key(&format!("::{head}"));
        !known
    }

    /// Resolve a class reference (`Dog`, `::Dog`, or a
    /// namespace-relative form) to its qualified name when it
    /// names a user-defined class.
    fn resolve_user_class(&self, name: &str) -> Option<String> {
        // Exact / canonical-global / unique-tail via the shared call-site
        // resolver — the former first-`HashMap`-hit `c.name == name` scan
        // picked an arbitrary same-tailed class across namespaces (M4.2).
        if let Some(q) =
            super::class_hierarchy::resolve_written_class_name(name, &self.result.all_classes)
        {
            return Some(q);
        }
        // Cross-file: a class defined elsewhere in the workspace (the oracle is
        // empty for the normal single-file analysis).  Exact / `::`-prefixed
        // spelling, else a *globally-unique* simple-name (tail) match — the same
        // abstain-on-ambiguity discipline the local tail match uses, so a name
        // shared by two workspace namespaces is left unresolved rather than
        // guessed.
        if self.workspace_classes.is_empty() {
            return None;
        }
        if self.workspace_classes.contains(name) {
            return Some(name.to_string());
        }
        // The same canonical global-qualified spelling the shared resolver
        // tries (colon-run rule, #934).
        let canonical = crate::naming::canonical_written_command(name);
        let qualified = if canonical.starts_with("::") {
            canonical
        } else {
            format!("::{canonical}")
        };
        if self.workspace_classes.contains(&qualified) {
            return Some(qualified);
        }
        let mut tail_hits = self
            .workspace_classes
            .iter()
            .filter(|q| crate::naming::key_tail(q) == name);
        if let Some(only) = tail_hits.next()
            && tail_hits.next().is_none()
        {
            return Some(only.clone());
        }
        None
    }

    /// Parse a `[CLASS new ...]` / `[CLASS create ...]`
    /// command-substitution value and return the qualified
    /// class name when `CLASS` is a user-defined class and the
    /// subcommand is a constructor (`new` / `create`).
    fn class_from_constructor_subst(&self, value: &str) -> Option<String> {
        let inner = value.trim();
        let inner = inner.strip_prefix('[')?.strip_suffix(']')?;
        let mut words = inner.split_whitespace();
        let class = words.next()?;
        // TclOO constructor: `set g [Class new|create ...]`.
        if let Some(subcmd) = words.next()
            && (subcmd == "new" || subcmd == "create")
            && let Some(uc) = self.resolve_user_class(class)
        {
            return Some(uc);
        }
        // Registry object-factory whose *return value* is an instance of its own
        // class — `set g [struct::graph]` / `set g [struct::graph name]`.  A
        // `creates_instance_at` spec marks a naming factory (`struct::graph
        // ?name?`) whose result is the object command, so the assigned var holds
        // an instance of `class` regardless of whether a name was passed.
        if let Some(reg) = self.registry
            && reg.object_class(class).is_some()
            && reg
                .get(class)
                .is_some_and(|s| s.creates_instance_at.is_some())
        {
            return Some(class.to_string());
        }
        None
    }

    /// When a `set VAR [... new|create ...]` constructor call's class head
    /// is a `$var` reference rather than the literal bareword
    /// [`Self::class_from_constructor_subst`] resolves directly, record a
    /// [`super::state::PendingInstanceClassSite`] so
    /// [`Self::settle_pending_instance_class_sites`] can bind it once the
    /// CFG/SSA flow-sensitive value model proves the variable's constant
    /// value (issue #923 idx 121: `set class ::Derived; set obj [$class
    /// create NAME]`, tcllib's `httpd/httpd.tcl`) — the same settle-late
    /// discipline `{*}$cmd` dispatch already uses (issue #945 / idx 94's
    /// `ConstDispatchSite`).  A no-op when the literal path already
    /// resolved the call, or the shape doesn't match.
    fn record_pending_instance_class_site(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
    ) {
        if cmd_name != "set"
            || args.len() < 2
            || self.class_from_constructor_subst(&args[1]).is_some()
        {
            return;
        }
        let Some(&arg_tok) = arg_tokens.get(1) else {
            return;
        };
        let Some((class_var, offset)) = class_var_head_constructor_subst(&args[1]) else {
            return;
        };
        let start = arg_tok.span.start() + offset;
        let len = 1 + u32::try_from(class_var.len()).unwrap_or(0);
        self.pending_instance_class_sites
            .push(super::state::PendingInstanceClassSite {
                class_var,
                span: Span::new(start, start + len),
                target_name: args[0].clone(),
            });
    }

    /// Settle every pending `$class`-headed `TclOO` instance-creation site
    /// (issue #923 idx 121) against `cu`'s flow-sensitive value model,
    /// mirroring [`Self::settle_const_dispatches`]'s settle-late
    /// discipline: `class_var`'s constant contributors are resolved to a
    /// user class exactly like [`Self::class_from_constructor_subst`]
    /// resolves a literal bareword, and bound into `instance_classes` only
    /// when every contributor agrees on the same single class — an
    /// unprovable value or a genuine (branch-dependent) ambiguity abstains
    /// soundly, leaving the pre-fix silence rather than guessing wrong.
    pub(in crate::analyser) fn settle_pending_instance_class_sites(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
    ) {
        if self.pending_instance_class_sites.is_empty() {
            return;
        }
        let sites = std::mem::take(&mut self.pending_instance_class_sites);
        for site in &sites {
            // A write trace can mutate the variable at any read — see
            // `settle_const_dispatches`'s identical guard.
            if cu.ir_module.has_dynamic_variable_trace
                || cu.ir_module.traced_variables.contains(&site.class_var)
            {
                continue;
            }
            let fu = cu.function_unit_at(site.span.start());
            let Some(contributors) =
                crate::value_provenance::const_contributors(fu, site.span.start(), &site.class_var)
            else {
                continue;
            };
            let mut resolved: Option<String> = None;
            for c in &contributors {
                let value = c.value.trim();
                if value.is_empty() || crate::naming::is_dynamic_word(value) {
                    resolved = None;
                    break;
                }
                let Some(qc) = self.resolve_user_class(value) else {
                    resolved = None;
                    break;
                };
                match &resolved {
                    None => resolved = Some(qc),
                    Some(existing) if *existing != qc => {
                        resolved = None;
                        break;
                    }
                    Some(_) => {}
                }
            }
            if let Some(class) = resolved {
                self.result
                    .instance_classes
                    .insert(site.target_name.clone(), class);
            }
        }
    }
}

/// Parse a `[$class new|create ...]` command-substitution value whose head
/// is a plain scalar-variable reference rather than the literal class
/// bareword [`Analyser::class_from_constructor_subst`] resolves directly —
/// returns the variable's name (no leading `$`) and its byte offset within
/// `value`, for the caller to anchor a
/// [`super::state::PendingInstanceClassSite`] (issue #923 idx 121).
/// `None` for anything but a bare `$name` head (no braces, array index, or
/// other computed shape) followed by `new`/`create` — the same "pure
/// reference" scope idx 94's `eval $cmd` fix uses, so a concatenated head
/// like `${class}Suffix` is left alone.
pub(in crate::analyser) fn class_var_head_constructor_subst(value: &str) -> Option<(String, u32)> {
    let inner = value.strip_prefix('[')?.strip_suffix(']')?;
    let lead = u32::try_from(inner.len() - inner.trim_start().len()).ok()?;
    let trimmed = inner.trim_start();
    let head_len = trimmed.find(char::is_whitespace)?;
    let head = &trimmed[..head_len];
    let rest = trimmed[head_len..].trim_start();
    let subcmd_len = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let subcmd = &rest[..subcmd_len];
    if subcmd != "new" && subcmd != "create" {
        return None;
    }
    let var_name = head.strip_prefix('$')?;
    if var_name.is_empty() || var_name.contains(['$', '[', ']', '{', '}', '(', ')']) {
        return None;
    }
    Some((var_name.to_string(), 1 + lead))
}

/// Whether `name` is a concrete, bindable instance-command name in a
/// `CLASS create NAME` construct — a plain word the analyser can register.
/// Excludes the auto-name token (`%AUTO%`), computed names (`$v`, `[…]`), and
/// any word carrying list / substitution metacharacters.
fn is_plain_created_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('%')
        && !name.contains(['$', '[', ']', '{', '}', '(', ')', ' ', '"'])
}

/// Descend a ``Cmd`` (``[…]``) substitution token and collect every
/// inner command's head as ``(name, head_span)``, recursing into nested
/// ``[...]`` substitutions and registry-resolved body arguments.
///
/// The token is descended into a child CST ([`descend_token`]) and the
/// inner script segmented; each command is then handled by
/// [`record_command_invocations`].  Spans are absolute (the descent
/// anchors the child tree at the substitution's position).
/// Statically-known argument count of a segmented command for the cross-file
/// arity check: `None` when any argument word is `{*}`-expanded (runtime count
/// unknown), else the literal argument count (`argv` minus the head).
fn segment_argc(seg: &SegmentedCommand) -> Option<usize> {
    if seg
        .expand_word
        .as_ref()
        .is_some_and(|e| e.iter().skip(1).any(|&x| x))
    {
        return None;
    }
    Some(seg.argv.len().saturating_sub(1))
}

/// The segment's first argument word, as an ensemble-subcommand candidate
/// `(text, span)` — `None` when there is no such word, it's `{*}`-expanded
/// (so the runtime subcommand isn't known statically), or it's otherwise a
/// dynamic word (issue #923 idx 106: nested-`[...]` counterpart of the
/// top-level check in `record_ensemble_subcommand_invocation`).
fn ensemble_subcommand_candidate(seg: &SegmentedCommand) -> Option<(String, Span)> {
    let expanded = seg
        .expand_word
        .as_ref()
        .and_then(|e| e.get(1))
        .copied()
        .unwrap_or(false);
    if expanded {
        return None;
    }
    let sub = seg.texts.get(1)?;
    if crate::naming::is_dynamic_word(sub) {
        return None;
    }
    let span = seg.argv.get(1)?.span;
    Some((sub.clone(), span))
}

fn collect_substitution_heads(
    sm: &SourceMap<'_>,
    registry: Option<&CommandRegistry>,
    cmd_tok: Token,
    config: LexerConfig,
    out: &mut Vec<CollectedHead>,
    expr_tokens: &mut Vec<Token>,
) {
    if cmd_tok.kind != TokenType::Cmd || sm.token_text(cmd_tok).is_empty() {
        return;
    }
    let descended = descend_token(sm, cmd_tok, config);
    for seg in segments_from_tree(descended.tree(), sm) {
        record_command_invocations(sm, registry, &seg, config, out, expr_tokens);
    }
}

/// Record one (already-segmented) command's head, then recurse into its
/// nested ``[...]`` substitutions *and* its registry-resolved body
/// arguments, so a command-substitution containing a control-flow
/// command surfaces the body's commands too (`[if {$c} {puts hi}]` →
/// `if`, `puts`).
///
/// The head is recorded as the `word_piece` form in `texts[0]`: a
/// ``$var`` head as ``${var}``, a ``"quoted"`` head unquoted, a compound
/// ``$x$y`` head reconstructed; a ``[subst]`` head is left to the
/// substitution recursion, and a ``{brace}`` head is data, not a
/// command.  Body arguments are resolved through [`descend_command`]
/// (the registry's ``arg_indices_for_role``), so the body set matches
/// the registry exactly and an ``Expr`` argument is never walked as a
/// script.
fn record_command_invocations(
    sm: &SourceMap<'_>,
    registry: Option<&CommandRegistry>,
    seg: &SegmentedCommand,
    config: LexerConfig,
    out: &mut Vec<CollectedHead>,
    expr_tokens: &mut Vec<Token>,
) {
    // Head — record the `word_piece` form in `texts[0]` for *every*
    // command, whatever the head's kind: a bare word, a `$var`
    // (`${var}`), a `"quote"` (unquoted), a compound head, a `[subst]`
    // head (`[gen]` — recorded *and* descended below), or a `{braced}`
    // head (its inner text).
    if let (Some(&head), Some(name)) = (seg.argv.first(), seg.texts.first())
        && !name.is_empty()
    {
        out.push((
            name.clone(),
            head.span,
            segment_argc(seg),
            None,
            ensemble_subcommand_candidate(seg),
        ));
    }
    // Command-prefix callback heads of this nested command (`return [lsort
    // -command myCompare $l]`): recorded with their appended arity so a
    // callback inside a `[...]` substitution is a first-class reference and is
    // arity-checked, exactly like a top-level one.
    if let (Some(registry), Some(name)) = (registry, seg.texts.first()) {
        let arg_texts: Vec<String> = seg.texts.iter().skip(1).cloned().collect();
        let arg_tokens: Vec<Token> = seg.argv.iter().skip(1).copied().collect();
        let arg_single: Vec<bool> = seg.single_token_word.iter().skip(1).copied().collect();
        for inv in crate::signature_scan::command_prefix::command_prefix_invocations(
            registry,
            name,
            &arg_texts,
            &arg_tokens,
            &arg_single,
        ) {
            out.push((inv.head, inv.span, None, Some(inv.appended), None));
        }
    }
    // Nested ``[...]`` substitutions in any position (args, or embedded
    // in a quoted word — both are `Cmd` tokens in the command's token
    // stream; a `{brace}` region stays opaque).
    for tok in &seg.all_tokens {
        if tok.kind == TokenType::Cmd {
            collect_substitution_heads(sm, registry, *tok, config, out, expr_tokens);
        }
    }
    // Registry-resolved body arguments (`if` / `foreach` / `eval` / …
    // bodies).  The body's commands are inner invocations too.
    if let (Some(registry), Some(name)) = (registry, seg.texts.first()) {
        let args: Vec<&str> = seg.texts.iter().skip(1).map(String::as_str).collect();
        let arg_tokens: Vec<Token> = seg.argv.iter().skip(1).copied().collect();
        // The `switch … {pattern body …}` list-form arg is a Tcl *list*,
        // not a script — the registry still marks it `Body`, but walking
        // it as one mis-reads a pattern as a command head.  It is
        // special-cased: parse the list
        // into pattern/body pairs and descend each arm *body*.
        let switch_list_idx = if name == "switch" {
            switch_list_body_index(&args)
        } else {
            None
        };
        // Definition commands are excluded from the plain-script body
        // descent for the same reason as `collect_segment_recursive`: their
        // bodies are definer grammars (member keywords, not commands), and
        // the real handlers walk them — recording `property`/`constructor`
        // here would draw spurious W123s.
        let bodies = if definition_handler_owns_body(registry, name) {
            Vec::new()
        } else {
            descend_command(registry, sm, name, &args, &arg_tokens, config)
        };
        for body in bodies {
            if switch_list_idx == Some(body.index) {
                let elements =
                    crate::segmenter::flatten_clause_list_elements(&body.text, body.token);
                // Elements alternate pattern, body, pattern, body, … —
                // descend the (odd-indexed) arm bodies only; a `-`
                // fall-through has no body of its own.
                let mut k = 1;
                while k < elements.len() {
                    let (arm_text, arm_tok) = &elements[k];
                    if arm_text != "-" && arm_tok.kind == TokenType::Str {
                        let arm = descend_token(sm, *arm_tok, config);
                        for inner in segments_from_tree(arm.tree(), sm) {
                            record_command_invocations(
                                sm,
                                Some(registry),
                                &inner,
                                config,
                                out,
                                expr_tokens,
                            );
                        }
                    }
                    k += 2;
                }
                continue;
            }
            for inner in segments_from_tree(body.descended.tree(), sm) {
                record_command_invocations(sm, Some(registry), &inner, config, out, expr_tokens);
            }
        }
        // Expr arguments (`if` / `while` / `expr` / … conditions): a
        // command substitution inside the expression is an invocation
        // too (`if {[acl_ok]} …` → `acl_ok`).  `descend_command`
        // deliberately excludes `Expr` args (they are not scripts), so
        // handle them here.  The expr token itself is also stashed in
        // `expr_tokens` so the caller can record its math-function
        // applications (`[expr {Pi()}]` → `::tcl::mathfunc::Pi`) — the
        // free-function path can't resolve `::tcl::mathfunc::` names, so
        // that resolution is deferred to the owning `&mut self` walk.
        for index in registry.arg_indices_for_role(name, &args, ArgRole::Expr) {
            if let Some(&tok) = arg_tokens.get(index) {
                expr_tokens.push(tok);
                collect_expr_substitutions(sm, Some(registry), tok, config, out, expr_tokens);
            }
        }
    }
}

/// Find the ``[…]`` command substitutions inside an expression argument
/// (`if` / `while` / `expr` conditions) and descend each.
///
/// An expression's own operands (`$x`, `+`, literals) are not commands,
/// so the braced expr is re-lexed as a script (which still tokenises a
/// ``[…]`` as a `Cmd`) and only the `Cmd` tokens are descended — never
/// the expression "head".
fn collect_expr_substitutions(
    sm: &SourceMap<'_>,
    registry: Option<&CommandRegistry>,
    expr_tok: Token,
    config: LexerConfig,
    out: &mut Vec<CollectedHead>,
    expr_tokens: &mut Vec<Token>,
) {
    if expr_tok.kind != TokenType::Str || sm.token_text(expr_tok).is_empty() {
        return;
    }
    let descended = descend_token(sm, expr_tok, config);
    for seg in segments_from_tree(descended.tree(), sm) {
        for tok in &seg.all_tokens {
            if tok.kind == TokenType::Cmd {
                collect_substitution_heads(sm, registry, *tok, config, out, expr_tokens);
            }
        }
    }
}

/// Descend a ``[…]`` substitution token and collect every command inside
/// it (recursing into nested ``[…]`` and the inner commands' bodies) into
/// `out`, for the caller to run the per-command dispatch on.  The bounds /
/// security companion of [`collect_substitution_heads`] — see
/// [`Analyser::run_nested_command_diagnostics`] for why a substitution is
/// the only region the main walk leaves unchecked.
fn collect_substitution_segments(
    sm: &SourceMap<'_>,
    registry: Option<&CommandRegistry>,
    cmd_tok: Token,
    config: LexerConfig,
    out: &mut Vec<SegmentedCommand>,
) {
    // Entry point: the outermost `[…]` substitution is bracket-nesting depth
    // 0 (issue #996 — the recursion cap lives in `collect_segment_recursive`,
    // which this and `collect_substitution_segments_at` mutually recurse
    // with).
    collect_substitution_segments_at(sm, registry, cmd_tok, config, out, 0);
}

fn collect_substitution_segments_at(
    sm: &SourceMap<'_>,
    registry: Option<&CommandRegistry>,
    cmd_tok: Token,
    config: LexerConfig,
    out: &mut Vec<SegmentedCommand>,
    depth: u32,
) {
    if cmd_tok.kind != TokenType::Cmd || sm.token_text(cmd_tok).is_empty() {
        return;
    }
    let descended = descend_token(sm, cmd_tok, config);
    for seg in segments_from_tree(descended.tree(), sm) {
        // A substitution's segments sit at the same nesting level as the
        // substitution itself; `collect_segment_recursive` enforces the cap.
        collect_segment_recursive(sm, registry, seg, config, out, depth);
    }
}

/// Recurse into one (already-segmented) substitution command's nested
/// ``[…]`` substitutions and registry-resolved bodies, then record the
/// command itself.  All of these live inside an outer ``[…]`` (the entry
/// is [`collect_substitution_segments`]), so none are visited by the main
/// walk and the dispatch never double-fires.
fn collect_segment_recursive(
    sm: &SourceMap<'_>,
    registry: Option<&CommandRegistry>,
    seg: SegmentedCommand,
    config: LexerConfig,
    out: &mut Vec<SegmentedCommand>,
    depth: u32,
) {
    // Native-stack safety net (issue #996): this and
    // `collect_substitution_segments_at` mutually recurse once per nested
    // `[…]` substitution / registry-resolved body inside a single word's raw
    // text — a genuinely unbounded axis. Past the cap, record this command
    // but stop descending into its nested substitutions/bodies: commands
    // buried deeper than the cap go unanalysed, never a crash.
    if MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
        out.push(seg);
        return;
    }
    // Nested ``[…]`` substitutions in any word of this command.
    for tok in &seg.all_tokens {
        if tok.kind == TokenType::Cmd {
            collect_substitution_segments_at(sm, registry, *tok, config, out, depth + 1);
        }
    }
    // Registry-resolved body arguments (`[if {$c} {string index …}]`):
    // their commands are also invisible to the main walk here.  Definition
    // commands are excluded: `dispatch_nested_segment` routes them through
    // the real proc/definer handlers, which own their body walks — a plain
    // script descent here would dispatch definer member keywords
    // (`property`, `constructor`) as unknown commands.
    if let Some(registry) = registry
        && !definition_handler_owns_body(registry, seg.name())
    {
        let args: Vec<&str> = seg.texts.iter().skip(1).map(String::as_str).collect();
        let arg_tokens: Vec<Token> = seg.argv.iter().skip(1).copied().collect();
        for body in descend_command(registry, sm, seg.name(), &args, &arg_tokens, config) {
            for inner in segments_from_tree(body.descended.tree(), sm) {
                collect_segment_recursive(sm, Some(registry), inner, config, out, depth + 1);
            }
        }
    }
    out.push(seg);
}

/// Whether the nested dispatch routes `name` through a real definition
/// handler (`handle_proc_command` / the class-definer handlers), which owns
/// its body walk — the generic collector must not also descend that body as
/// a plain script.  Registry-driven: a `definition_body` grammar (class
/// definers, `oo::define`) or the `DEFINES_PROCEDURE` trait (`proc`).
fn definition_handler_owns_body(registry: &CommandRegistry, name: &str) -> bool {
    registry.get(name).is_some_and(|spec| {
        spec.definition_body.is_some()
            || spec
                .traits
                .contains(tcl_registry::Traits::DEFINES_PROCEDURE)
    })
}

/// For the ``switch ?options? string {pattern body …}`` form, the index
/// (into `args`, 0-based, excluding the command name) of the single
/// braced pattern/body list.  `None` for the separate-args form
/// (``switch string pat body pat body``), whose bodies *are* scripts.
fn switch_list_body_index(args: &[&str]) -> Option<usize> {
    let mut i = 0;
    while i < args.len() && args[i].starts_with('-') {
        if args[i] == "--" {
            i += 1;
            break;
        }
        i += 1;
    }
    if i >= args.len() {
        return None;
    }
    i += 1; // the `string` argument
    if i == args.len() - 1 { Some(i) } else { None }
}

/// Every arm *body* of a `switch ?options? string {pattern body …}` /
/// `switch ?options? string pattern body pattern body …` command, in
/// source order — the `-` fall-through marker (a body borrowed from the
/// next arm, not one of its own) is skipped. `args`/`arg_tokens` exclude
/// the command name (`args[0]` is the first option or the `string` word),
/// matching every other command handler's own convention.
///
/// Handles both of `switch`'s argument shapes, matching
/// [`Analyser::handle_switch_command`]'s own form-1/form-2 branching
/// exactly (kept independent of it — this returns only the body list a
/// caller needs for scanning, not the pattern-recording side effects
/// `handle_switch_command` also performs, e.g. `-regexp` pattern
/// diagnostics), so the two can never disagree about where an arm's body
/// starts and ends.
///
/// Consumed by `tcl_lsp_core::references::scan_my_method_region` to
/// descend into a switch-dispatched `TclOO` method body's arms when
/// searching for `my method` call sites (issue #923 idx 63, main audit
/// wave: the "assigned `Add`-dispatcher" idiom — `switch ... { barSeries
/// { my AddBarSeries {*}$args } ... }` — mines exactly this shape from
/// nico-robert/ticklecharts).
#[must_use]
pub fn switch_arm_bodies(args: &[String], arg_tokens: &[Token]) -> Vec<(String, Token)> {
    let mut out = Vec::new();
    if args.len() < 2 {
        return out;
    }
    let mut i = 0;
    while i < args.len() && args[i].starts_with('-') {
        if args[i] == "--" {
            i += 1;
            break;
        }
        i += 1;
    }
    i += 1; // the `string` argument
    if i >= args.len() {
        return out;
    }
    if i == args.len() - 1 {
        // Form 2 — single braced body containing all pairs.
        let Some(body_tok) = arg_tokens.get(i).copied() else {
            return out;
        };
        let elements = crate::segmenter::flatten_clause_list_elements(&args[i], body_tok);
        let mut j = 1;
        while j < elements.len() {
            let (body_text, body_tok) = &elements[j];
            if body_text != "-" {
                out.push((body_text.clone(), *body_tok));
            }
            j += 2;
        }
    } else {
        // Form 1 — pattern/body pairs inline in args/arg_tokens.
        while i + 1 < args.len() {
            let body_text = &args[i + 1];
            if let Some(body_tok) = arg_tokens.get(i + 1).copied()
                && body_text != "-"
            {
                out.push((body_text.clone(), body_tok));
            }
            i += 2;
        }
    }
    out
}

/// Top-level ``[...]`` command-substitution regions in `text`, as
/// `(inner_byte_offset, inner_text)` — the script *inside* the brackets and
/// the offset of its first byte within `text`.  ``\\[`` / ``\\]`` escapes are
/// honoured.  Only the *outermost* substitutions are returned; the caller's
/// segment recursion descends any nested `[...]`.  Used to reach `[...]`
/// embedded in a quoted / bareword word argument (`log "got [HTTP::uri]"`),
/// which the bare-`Cmd`-token walk misses.
///
/// Braces are **not** treated as opaque here: this only runs on `Esc` words
/// (quoted / bareword / compound).  A braced *word* is a `Str` token (handled
/// elsewhere and excluded by the caller); inside a quoted or bareword context
/// `{` / `}` are ordinary characters that do *not* suppress substitution, so
/// `log "got { [HTTP::uri] }"` still executes — and must still be scanned.
fn top_level_cmd_subst_regions(text: &str) -> Vec<(usize, &str)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'[' => {
                let inner_start = i + 1;
                let mut depth = 1i32;
                let mut j = inner_start;
                while j < bytes.len() && depth > 0 {
                    match bytes[j] {
                        b'[' => depth += 1,
                        b']' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        b'\\' if j + 1 < bytes.len() => j += 1,
                        _ => {}
                    }
                    j += 1;
                }
                if depth == 0 && j <= bytes.len() {
                    out.push((inner_start, &text[inner_start..j]));
                    i = j + 1;
                } else {
                    i += 1;
                }
            }
            b'\\' if i + 1 < bytes.len() => i += 2,
            _ => i += 1,
        }
    }
    out
}

/// Walk `text` looking for ``[cmd args...]`` command
/// substitutions and return the head name + the offset of the
/// head's first byte within `text` for each substitution found.
/// Nested substitutions are reported in depth-first order
/// (outer first, then inner).  Braced regions are skipped
/// opaquely; backslash-escaped ``\\[`` / ``\\]`` are skipped.
///
/// Returns ``(name, byte_offset_in_text)`` pairs.  The caller
/// adds the enclosing token's source-span start to obtain an
/// absolute offset.
pub(crate) fn scan_nested_command_heads(text: &str) -> Vec<(String, u32)> {
    // Entry point: the outermost word's raw text is bracket-nesting depth 0
    // (issue #996 — the recursion cap lives in
    // [`scan_nested_command_heads_at`]).
    scan_nested_command_heads_at(text, 0)
}

fn scan_nested_command_heads_at(text: &str, rec_depth: u32) -> Vec<(String, u32)> {
    // Native-stack safety net (issue #996): this self-recurses once per
    // nested `[…]` substitution inside a single word's raw text — a genuinely
    // unbounded axis. Past the cap, return what's been found so far: nested
    // heads buried deeper than the cap go unreported, never a crash.
    // (`rec_depth` is named apart from the local `[`/`{`-matching `depth`
    // counters below, which track bracket balance, not native recursion.)
    if MAX_BRACKET_TEXT_DEPTH.exceeded(rec_depth) {
        return Vec::new();
    }
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                // Skip braced region opaquely.
                let mut depth = 1i32;
                i += 1;
                while i < bytes.len() && depth > 0 {
                    match bytes[i] {
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        b'\\' if i + 1 < bytes.len() => i += 1,
                        _ => {}
                    }
                    i += 1;
                }
            }
            b'[' => {
                // Find the matching closing ``]`` honouring
                // nesting + backslash escapes.
                let inner_start = i + 1;
                let mut depth = 1i32;
                let mut j = inner_start;
                while j < bytes.len() && depth > 0 {
                    match bytes[j] {
                        b'[' => depth += 1,
                        b']' => depth -= 1,
                        b'\\' if j + 1 < bytes.len() => j += 1,
                        _ => {}
                    }
                    if depth == 0 {
                        break;
                    }
                    j += 1;
                }
                if depth == 0 && j < bytes.len() {
                    let inner = &text[inner_start..j];
                    let inner_start_u32 = u32::try_from(inner_start)
                        .expect("byte offset fits in u32 for in-memory source");
                    if let Some((name, head_offset_in_inner)) = first_command_head(inner) {
                        let abs_offset = inner_start_u32
                            + u32::try_from(head_offset_in_inner)
                                .expect("inner offset fits in u32 for in-memory source");
                        out.push((name.to_string(), abs_offset));
                    }
                    // Recurse into the inner text — nested ``[...]``
                    // substitutions inside this one also produce
                    // invocations.
                    for (name, off_in_inner) in scan_nested_command_heads_at(inner, rec_depth + 1) {
                        out.push((name, inner_start_u32 + off_in_inner));
                    }
                    i = j + 1;
                } else {
                    i += 1;
                }
            }
            b'\\' if i + 1 < bytes.len() => i += 2,
            _ => i += 1,
        }
    }
    out
}

/// Find the first command-head token in `text` (skipping
/// leading whitespace and comments) and return its ``(name,
/// offset)`` pair.  Conservative — any non-bareword leading
/// token returns ``None``.
fn first_command_head(text: &str) -> Option<(&str, usize)> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == b';' {
            i += 1;
            continue;
        }
        // Skip comment lines if at the start of a logical command.
        if c == b'#' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        break;
    }
    if i >= bytes.len() {
        return None;
    }
    let start = i;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b' '
            || c == b'\t'
            || c == b'\n'
            || c == b'\r'
            || c == b';'
            || c == b'['
            || c == b']'
        {
            break;
        }
        i += 1;
    }
    if i == start {
        return None;
    }
    let head = &text[start..i];
    // Reject heads that look like substitution / quoting markers
    // (``$foo``, ``"abc"``, ``{...}``) — these aren't command
    // names.
    if head.starts_with('$') || head.starts_with('"') || head.starts_with('{') {
        return None;
    }
    Some((head, start))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{Span, TokenType};

    /// The dispatch-level replacement for the retired per-handler
    /// name-guard tests (`handle_set_wrong_command_no_op` and
    /// friends): an unstamped or `::`-qualified head resolves no
    /// [`tcl_registry::hooks::AnalyserHookId`], so no handler runs at
    /// all, while stamped heads resolve the family the guards used to
    /// select — including the subcommand-level stamps.  Runs on a bare
    /// `Analyser::new()`, which also exercises the shared
    /// [`fallback_registry`] path the unit harnesses rely on.
    /// `if {1} { if {1} { ... } }`, `depth` levels deep, wrapped in a `proc`
    /// body.
    fn nested_if_source(depth: u32) -> String {
        let mut source = String::from("proc deepnest {} {\n");
        for _ in 0..depth {
            source.push_str("if {1} {\n");
        }
        for _ in 0..depth {
            source.push_str("}\n");
        }
        source.push_str("}\n");
        source
    }

    /// Run `analyse` on a dedicated thread with a generous stack.
    ///
    /// `cargo test` runs each `#[test]` on its own thread with the platform
    /// default stack size (~2 MiB on Linux) — the same undersized budget
    /// that caused issue #996's crash in the first place (Tokio's default
    /// worker-thread stack is the same size). A test that walks source
    /// nested past [`MAX_BODY_DEPTH`] needs the same generous, explicit
    /// stack production code now gets via `tokio::runtime::Builder::
    /// thread_stack_size` (`tcl-lsp-server`/`tcl-mcp`) and `std::thread::
    /// Builder::stack_size` (the `tcl` CLI) — otherwise the test harness
    /// itself hits the bug this suite exists to catch.
    fn analyse_on_big_stack(source: String, dialect: &str) -> super::super::types::AnalysisResult {
        let dialect = dialect.to_owned();
        std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(move || Analyser::new().analyse(&source, &dialect))
            .expect("spawn big-stack test thread")
            .join()
            .expect("analyse on big-stack thread panicked")
    }

    #[test]
    fn depth_exactly_at_cap_emits_no_e207() {
        // The wrapping `proc` body is itself one level of `body_depth`, so
        // `MAX_BODY_DEPTH - 1` nested `if`s is what brings body_depth to
        // exactly `MAX_BODY_DEPTH`.
        let source = nested_if_source(MAX_BODY_DEPTH.0 - 1);
        let res = analyse_on_big_stack(source, "tcl9.0");
        assert!(
            !res.diagnostics.iter().any(|d| d.code == DiagCode::E207),
            "depth == MAX_BODY_DEPTH must not trip the cap: {:?}",
            res.diagnostics.iter().map(|d| d.code).collect::<Vec<_>>()
        );
    }

    #[test]
    fn depth_one_past_cap_emits_e207_exactly_once() {
        let source = nested_if_source(MAX_BODY_DEPTH.0);
        let res = analyse_on_big_stack(source, "tcl9.0");
        let e207_count = res
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::E207)
            .count();
        assert_eq!(
            e207_count, 1,
            "depth == MAX_BODY_DEPTH + 1 must trip the cap exactly once, got {e207_count}"
        );
    }

    #[test]
    fn depth_far_past_cap_still_emits_e207_exactly_once() {
        // Not flooded: the cap trips at the same nesting level every time
        // (once body_depth cannot go higher), so it must still be exactly
        // one diagnostic at 2000 levels, not one per level past the cap.
        let source = nested_if_source(2000);
        let res = analyse_on_big_stack(source, "tcl9.0");
        let e207_count = res
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::E207)
            .count();
        assert_eq!(
            e207_count, 1,
            "2000 levels must still trip the cap exactly once, got {e207_count}"
        );
    }

    #[test]
    fn shallow_nesting_never_emits_e207() {
        // False-positive guard: ordinary, hand-written nesting must never
        // draw E207.
        let source = nested_if_source(10);
        let mut a = Analyser::new();
        let res = a.analyse(&source, "tcl9.0");
        assert!(
            !res.diagnostics.iter().any(|d| d.code == DiagCode::E207),
            "shallow nesting must never emit E207: {:?}",
            res.diagnostics.iter().map(|d| d.code).collect::<Vec<_>>()
        );
    }

    #[test]
    fn resolve_analyser_hook_mirrors_the_retired_name_guards() {
        use tcl_registry::hooks::AnalyserHookId as H;
        let a = Analyser::new();
        let args = |words: &[&str]| words.iter().map(ToString::to_string).collect::<Vec<_>>();

        // Unstamped head: no handler family.
        assert_eq!(a.resolve_analyser_hook("puts", &args(&["hi"])), None);
        // Command-level stamp.
        assert_eq!(
            a.resolve_analyser_hook("proc", &args(&["p", "a", "b"])),
            Some(H::Proc)
        );
        // Subcommand-level stamps pick the family per subcommand word…
        assert_eq!(
            a.resolve_analyser_hook("namespace", &args(&["eval", "ns", "{}"])),
            Some(H::NamespaceEval)
        );
        assert_eq!(
            a.resolve_analyser_hook("namespace", &args(&["import", "::t::*"])),
            Some(H::NamespaceImport)
        );
        // …and an unstamped subcommand of a stamped command resolves
        // nothing (`namespace which`, `dict set`).
        assert_eq!(
            a.resolve_analyser_hook("namespace", &args(&["which", "x"])),
            None
        );
        assert_eq!(
            a.resolve_analyser_hook("dict", &args(&["set", "d", "k", "v"])),
            None
        );
        assert_eq!(
            a.resolve_analyser_hook("dict", &args(&["for", "{k v}", "$d", "{}"])),
            Some(H::DictFor)
        );
        // A **namespaced** registry spelling is different (issue #923 idx
        // 90): real corpus code commonly writes `::tcl::OptProc` fully
        // qualified, and `CommandRegistry::get`'s single-`::`-strip
        // fallback resolves that to the exact same spec either way — an
        // exact match, never the naive tail-matching the bareword guard
        // above exists to rule out. Both spellings resolve identically…
        assert_eq!(
            a.resolve_analyser_hook("tcl::OptProc", &args(&["p", "{a}", "{}"])),
            Some(H::OptProc)
        );
        assert_eq!(
            a.resolve_analyser_hook("::tcl::OptProc", &args(&["p", "{a}", "{}"])),
            Some(H::OptProc)
        );
        // …while a `::`-qualified spelling of a genuinely bareword global
        // command is still blocked — the guard narrows on "does the
        // bareword tail itself contain `::`", not merely "starts with
        // `::`", so this pinned case keeps its exact prior behaviour.
        assert_eq!(
            a.resolve_analyser_hook("::proc", &args(&["p", "a", "b"])),
            None
        );
    }

    /// A Tk widget constructor is syntactically identical to a tcllib
    /// naming factory (`struct::graph g`) — a bareword `ttk::treeview .t`
    /// must bind `.t` in `instance_classes`/`created_instance_commands`
    /// through the existing, already-generic `record_registry_factory_instance`
    /// with no new analyser code, once the registry declares
    /// `creates_instance_at`/`object_class` (issue #927).
    #[test]
    fn bareword_widget_constructor_binds_instance_class() {
        let mut a = super::super::state::Analyser::new();
        let res = a.analyse("ttk::treeview .t\n.t instate {selected} {}\n", "tcl8.6");
        assert_eq!(
            res.instance_classes.get(".t").map(String::as_str),
            Some("ttk::treeview"),
            "instance_classes: {:?}",
            res.instance_classes
        );
        assert!(
            res.created_instance_commands.contains(".t"),
            "created_instance_commands: {:?}",
            res.created_instance_commands
        );
    }

    /// The `set w [ctor .path]` return-value-capture shape resolves through
    /// the existing, already-generic `registry_factory_class_from_subst` —
    /// again no new analyser code needed once the registry data lands.
    #[test]
    fn var_captured_widget_constructor_binds_instance_class() {
        let mut a = super::super::state::Analyser::new();
        let res = a.analyse("set lb [listbox .l]\n$lb curselection\n", "tcl8.6");
        assert_eq!(
            res.instance_classes.get("lb").map(String::as_str),
            Some("listbox"),
            "instance_classes: {:?}",
            res.instance_classes
        );
    }

    /// A plain widget with no modelled subcommands (`button`) still binds
    /// an instance class (useful for definition/references/W123
    /// suppression) even though it has no `object_class` to dispatch
    /// methods against.
    #[test]
    fn simple_widget_without_subcommands_still_binds_instance_class() {
        let mut a = super::super::state::Analyser::new();
        let res = a.analyse("button .b -text hi\n", "tcl8.6");
        assert_eq!(
            res.instance_classes.get(".b").map(String::as_str),
            Some("button"),
            "instance_classes: {:?}",
            res.instance_classes
        );
    }

    #[test]
    fn interp_create_safe_with_literal_name_registers_the_command() {
        // TP — regression for a bug found by differential audit against
        // docstrip_util.tcl: `interp create -safe NAME` (or `-- NAME`) read
        // `defines_command_at`'s fixed index straight into the raw sub-args
        // without skipping the leading option words first, so it landed on
        // `-safe` itself, saw it start with `-`, and recorded nothing —
        // leaving every later literal call to `NAME` a false-positive W123.
        let mut a = super::super::state::Analyser::new();
        let res = a.analyse("interp create -safe sandbox\n", "tcl9.0");
        assert!(
            res.created_instance_commands.contains("sandbox"),
            "created_instance_commands: {:?}",
            res.created_instance_commands
        );
    }

    #[test]
    fn interp_create_double_dash_with_literal_name_registers_the_command() {
        // TP — the `--` option-terminator form of the same idiom.
        let mut a = super::super::state::Analyser::new();
        let res = a.analyse("interp create -- sandbox\n", "tcl9.0");
        assert!(
            res.created_instance_commands.contains("sandbox"),
            "created_instance_commands: {:?}",
            res.created_instance_commands
        );
    }

    #[test]
    fn interp_create_double_dash_with_option_shaped_literal_name_registers_the_command() {
        // TP — regression for a bug found by Codex review of PR #963:
        // `leading_option_word_count` kept matching option-shaped words
        // *after* a `--` terminator, so `interp create -- -safe` (which real
        // `tclsh9.0` accepts and names the child interpreter literally
        // `-safe` — verified empirically: `interp create -- -safe` then
        // `info commands -safe` lists it) wrongly consumed `-safe` too,
        // as if it were the `-safe` flag rather than the positional name.
        let mut a = super::super::state::Analyser::new();
        let res = a.analyse("interp create -- -safe\n", "tcl9.0");
        assert!(
            res.created_instance_commands.contains("-safe"),
            "created_instance_commands: {:?}",
            res.created_instance_commands
        );
    }

    #[test]
    fn interp_create_safe_and_double_dash_with_literal_name_registers_the_command() {
        // TP — both option words stacked (`-safe --`), each consumed once.
        let mut a = super::super::state::Analyser::new();
        let res = a.analyse("interp create -safe -- sandbox\n", "tcl9.0");
        assert!(
            res.created_instance_commands.contains("sandbox"),
            "created_instance_commands: {:?}",
            res.created_instance_commands
        );
    }

    #[test]
    fn interp_create_with_no_flags_still_registers_the_command() {
        // FN guard — the pre-existing, always-worked shape (`interp create
        // NAME`, no leading options at all) must not regress: with an empty
        // leading-option skip, `idx` must land exactly where it always did.
        let mut a = super::super::state::Analyser::new();
        let res = a.analyse("interp create sandbox\n", "tcl9.0");
        assert!(
            res.created_instance_commands.contains("sandbox"),
            "created_instance_commands: {:?}",
            res.created_instance_commands
        );
    }

    #[test]
    fn interp_create_safe_with_literal_name_suppresses_w123_on_later_call() {
        // TP — the user-visible symptom: before the fix, every literal call
        // to the interpreter's own object command raised a false-positive
        // W123 "unknown command", since the name was never recorded at all.
        let src = "interp create -safe sandbox\nsandbox eval {set x 1}\n";
        assert!(
            !has_code(src, "tcl9.0", "W123"),
            "a literal call to a `interp create -safe`-created name must \
             not raise W123: {:?}",
            diag_codes(src, "tcl9.0")
        );
    }

    #[test]
    fn interp_create_with_no_name_records_nothing() {
        // TN — a missing name auto-generates one at run time; there is no
        // literal text to record, and the (former) `-safe`-as-name misread
        // must not resurface as some other spurious entry.
        let mut a = super::super::state::Analyser::new();
        let res = a.analyse("interp create -safe\n", "tcl9.0");
        assert!(
            res.created_instance_commands.is_empty(),
            "created_instance_commands: {:?}",
            res.created_instance_commands
        );
    }

    #[test]
    fn scan_nested_command_heads_simple() {
        let out = scan_nested_command_heads("[helper $x]");
        assert_eq!(out, vec![("helper".to_string(), 1)]);
    }

    #[test]
    fn scan_nested_command_heads_nested() {
        // [outer [inner $x]]
        let out = scan_nested_command_heads("[outer [inner $x]]");
        assert_eq!(
            out,
            vec![("outer".to_string(), 1), ("inner".to_string(), 8)]
        );
    }

    /// Regression coverage for issue #996: `scan_nested_command_heads`
    /// self-recurses once per nested `[…]` substitution inside a single
    /// word's raw text (Tier 1B), and `collect_segment_recursive` /
    /// `collect_substitution_segments` mutually recurse the same way. Both
    /// were genuinely unbounded before this fix, independent of the
    /// statement-tree `MAX_BODY_DEPTH` cap, and empirically overflowed the
    /// native stack (SIGABRT) in the low thousands of levels on a 2 MiB
    /// thread. 3000 is past that crash range and past `MAX_BRACKET_TEXT_DEPTH`
    /// (256); the assertion is that each returns.
    #[test]
    fn deeply_nested_command_head_scans_survive() {
        // `[a [a [a … [a x] … ]]]` nested bracket substitutions drive the
        // direct Tier 1B text scanner.
        let mut brackets = "x".to_owned();
        for _ in 0..3000 {
            brackets = format!("[a {brackets}]");
        }
        let _ = scan_nested_command_heads(&brackets);

        // `a [a [a … [x] … ]]` — each level is a literal-headed command whose
        // argument holds the next `[…]` substitution — drives the
        // `collect_segment_recursive` / `collect_substitution_segments` mutual
        // recursion directly (isolated from the full analyser pipeline).
        let mut nested = "x".to_owned();
        for _ in 0..3000 {
            nested = format!("a [{nested}]");
        }
        let config = LexerConfig::for_dialect("tcl8.6");
        let sm = SourceMap::new(&nested);
        let reg = CommandRegistry::build_default();
        let mut out = Vec::new();
        for seg in crate::segmenter::segment_commands_with_offset_and_config(&nested, 0, config) {
            collect_segment_recursive(&sm, Some(&reg), seg, config, &mut out, 0);
        }
    }

    #[test]
    fn scan_nested_command_heads_inside_quoted_string() {
        // "got [count $items]" — quotes don't interfere with [
        // ``count`` starts at byte 6 (after ``"got [``).
        let out = scan_nested_command_heads("\"got [count $items]\"");
        assert_eq!(out, vec![("count".to_string(), 6)]);
    }

    #[test]
    fn scan_nested_command_heads_skips_braced_regions() {
        // Braced regions are opaque — `[` inside `{...}` doesn't count.
        // ``real`` starts at byte 15 (after ``{[not_a_cmd]} [``).
        let out = scan_nested_command_heads("{[not_a_cmd]} [real]");
        assert_eq!(out, vec![("real".to_string(), 15)]);
    }

    #[test]
    fn scan_nested_command_heads_skips_backslash_escape() {
        // \[foo\] is a literal pair, not a substitution.
        // ``bar`` starts at byte 9 (after ``\\[foo\\] [``).
        let out = scan_nested_command_heads("\\[foo\\] [bar]");
        assert_eq!(out, vec![("bar".to_string(), 9)]);
    }

    #[test]
    fn scan_nested_command_heads_no_match_for_pure_var_head() {
        // [$cmd args] — head is a variable substitution, not a name
        let out = scan_nested_command_heads("[$cmd args]");
        assert!(out.is_empty());
    }

    #[test]
    fn scan_nested_command_heads_no_match_for_unclosed() {
        // Unclosed [ — returns nothing (recovery is segmenter's job)
        let out = scan_nested_command_heads("[lindex $x 0");
        assert!(out.is_empty());
    }

    #[test]
    fn scan_nested_command_heads_two_independent_substs() {
        // [a $x] [b $y]
        let out = scan_nested_command_heads("[a $x] [b $y]");
        assert_eq!(out, vec![("a".to_string(), 1), ("b".to_string(), 8)]);
    }

    fn esc_tok(span: Span) -> Token {
        Token::new(TokenType::Esc, span)
    }

    fn str_tok(span: Span) -> Token {
        Token {
            kind: TokenType::Str,
            span,
            content_offset: 1,
            in_quote: false,
        }
    }

    fn span(start: u32, end: u32) -> Span {
        Span::new(start, end)
    }

    // switch_arm_bodies

    #[test]
    fn switch_arm_bodies_form1_extracts_each_body() {
        // Form 1: `switch $x a {set y 1} b {set z 2}` — the same shape
        // `handle_switch_form1_walks_each_arm_body` (handlers.rs) proves
        // gets walked; this pins the extraction function itself
        // (issue #923 idx 63, main audit wave).
        let args = vec![
            "$x".to_string(),
            "a".to_string(),
            "set y 1".to_string(),
            "b".to_string(),
            "set z 2".to_string(),
        ];
        let arg_tokens = vec![
            esc_tok(span(7, 9)),
            esc_tok(span(10, 11)),
            str_tok(span(13, 22)),
            esc_tok(span(24, 25)),
            str_tok(span(27, 36)),
        ];
        let bodies = switch_arm_bodies(&args, &arg_tokens);
        let texts: Vec<&str> = bodies.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(texts, vec!["set y 1", "set z 2"]);
    }

    #[test]
    fn switch_arm_bodies_form2_extracts_each_body_from_the_braced_list() {
        // Form 2: `switch $x { a {set y 1} b {set z 2} }` — the same
        // shape `handle_switch_form2_braced_body_walks_each_arm`
        // (handlers.rs) proves gets walked.
        let body_text = " a {set y 1} b {set z 2} ".to_string();
        let args = vec!["$x".to_string(), body_text];
        let arg_tokens = vec![esc_tok(span(7, 9)), str_tok(span(10, 37))];
        let bodies = switch_arm_bodies(&args, &arg_tokens);
        let texts: Vec<&str> = bodies.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(texts, vec!["set y 1", "set z 2"]);
    }

    #[test]
    fn switch_arm_bodies_skips_the_fallthrough_marker() {
        // `switch $x a - b {set y 1}` — the `-` body for pattern `a` is
        // fall-through (the next arm's body actually runs), so it must
        // not appear as a body of its own.
        let args = vec![
            "$x".to_string(),
            "a".to_string(),
            "-".to_string(),
            "b".to_string(),
            "set y 1".to_string(),
        ];
        let arg_tokens = vec![
            esc_tok(span(7, 9)),
            esc_tok(span(10, 11)),
            esc_tok(span(12, 13)),
            esc_tok(span(14, 15)),
            str_tok(span(17, 26)),
        ];
        let bodies = switch_arm_bodies(&args, &arg_tokens);
        let texts: Vec<&str> = bodies.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(texts, vec!["set y 1"]);
    }

    #[test]
    fn switch_arm_bodies_too_few_args_is_empty() {
        assert!(switch_arm_bodies(&["$x".to_string()], &[]).is_empty());
        assert!(switch_arm_bodies(&[], &[]).is_empty());
    }

    #[test]
    fn process_set_defines_variable() {
        let mut a = Analyser::new();
        a.process_command(
            &["set".to_string(), "x".to_string(), "1".to_string()],
            &[
                esc_tok(span(0, 3)),
                esc_tok(span(4, 5)),
                esc_tok(span(6, 7)),
            ],
            &[true, true, true],
            &[],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("x"));
    }

    #[test]
    fn process_proc_records_at_global() {
        let mut a = Analyser::new();
        a.process_command(
            &[
                "proc".to_string(),
                "foo".to_string(),
                "a b".to_string(),
                "set x $a".to_string(),
            ],
            &[
                esc_tok(span(0, 4)),
                esc_tok(span(5, 8)),
                esc_tok(span(9, 14)),
                str_tok(span(15, 25)),
            ],
            &[true, true, true, true],
            &[],
            &[],
        );
        assert!(a.result.all_procs.contains_key("::foo"));
    }

    #[test]
    fn process_namespace_eval_opens_scope() {
        let mut a = Analyser::new();
        a.process_command(
            &[
                "namespace".to_string(),
                "eval".to_string(),
                "ns1".to_string(),
                String::new(),
            ],
            &[
                esc_tok(span(0, 9)),
                esc_tok(span(10, 14)),
                esc_tok(span(15, 18)),
                str_tok(span(19, 21)),
            ],
            &[true, true, true, true],
            &[],
            &[],
        );
        assert_eq!(a.result.global_scope.children.len(), 1);
        assert_eq!(a.result.global_scope.children[0].name, "ns1");
    }

    #[test]
    fn process_foreach_defines_loop_var() {
        let mut a = Analyser::new();
        a.process_command(
            &[
                "foreach".to_string(),
                "i".to_string(),
                "{1 2 3}".to_string(),
                "puts $i".to_string(),
            ],
            &[
                esc_tok(span(0, 7)),
                esc_tok(span(8, 9)),
                str_tok(span(10, 17)),
                str_tok(span(18, 28)),
            ],
            &[true, true, true, true],
            &[],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("i"));
    }

    #[test]
    fn process_global_defines_each_name() {
        let mut a = Analyser::new();
        a.process_command(
            &["global".to_string(), "x".to_string(), "y".to_string()],
            &[
                esc_tok(span(0, 6)),
                esc_tok(span(7, 8)),
                esc_tok(span(9, 10)),
            ],
            &[true, true, true],
            &[],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("x"));
        assert!(a.result.global_scope.variables.contains_key("y"));
    }

    #[test]
    fn process_unknown_command_silently_no_op() {
        let mut a = Analyser::new();
        a.process_command(
            &["my_unknown_command".to_string(), "arg".to_string()],
            &[esc_tok(span(0, 18)), esc_tok(span(19, 22))],
            &[true, true],
            &[],
            &[],
        );
        // No handler matched; no procs, vars, classes, or aliases
        // recorded.
        assert!(a.result.all_procs.is_empty());
        assert!(a.result.global_scope.variables.is_empty());
    }

    #[test]
    fn process_empty_argv_is_no_op() {
        let mut a = Analyser::new();
        a.process_command(&[], &[], &[], &[], &[]);
        // No panic, no state mutation.
    }

    #[test]
    fn process_interp_alias_records_target() {
        let mut a = Analyser::new();
        a.process_command(
            &[
                "interp".to_string(),
                "alias".to_string(),
                String::new(),
                "myset".to_string(),
                String::new(),
                "set".to_string(),
            ],
            &[
                esc_tok(span(0, 6)),
                esc_tok(span(7, 12)),
                str_tok(span(13, 15)),
                esc_tok(span(16, 21)),
                str_tok(span(22, 24)),
                esc_tok(span(25, 28)),
            ],
            &[true, true, true, true, true, true],
            &[],
            &[],
        );
        assert!(a.command_aliases.contains_key("::myset"));
    }

    fn diag_codes(source: &str, dialect: &str) -> Vec<(String, String)> {
        let mut a = Analyser::new();
        let res = a.analyse(source, dialect);
        res.diagnostics
            .iter()
            .map(|d| (d.code.to_string(), d.message.clone()))
            .collect()
    }

    fn has_code(source: &str, dialect: &str, code: &str) -> bool {
        diag_codes(source, dialect).iter().any(|(c, _)| c == code)
    }

    #[test]
    fn orphaned_keyword_parent_maps_keywords() {
        assert_eq!(orphaned_keyword_parent("else"), Some("if"));
        assert_eq!(orphaned_keyword_parent("elseif"), Some("if"));
        assert_eq!(orphaned_keyword_parent("then"), Some("if"));
        assert_eq!(orphaned_keyword_parent("on"), Some("try"));
        assert_eq!(orphaned_keyword_parent("trap"), Some("try"));
        assert_eq!(orphaned_keyword_parent("finally"), Some("try"));
        assert_eq!(orphaned_keyword_parent("set"), None);
    }

    #[test]
    fn w125_fires_for_orphaned_else() {
        // A misplaced newline split the `if` — `else` lands as a standalone
        // command with no parent.
        assert!(has_code(
            "if {$x} {\n  puts hi\n}\nelse {\n  puts bye\n}",
            "tcl",
            "W125"
        ));
    }

    #[test]
    fn w125_quiet_for_attached_else() {
        // Properly attached `} else {` never segments `else` as a command.
        assert!(!has_code(
            "if {$x} {\n  puts hi\n} else {\n  puts bye\n}",
            "tcl",
            "W125"
        ));
    }

    #[test]
    fn w125_suppressed_when_user_proc_shadows_keyword() {
        // A user proc named `finally` shadows the keyword — no warning.
        assert!(!has_code(
            "proc finally {} { return }\nfinally",
            "tcl",
            "W125"
        ));
    }

    #[test]
    fn irule5005_fires_for_direct_proc_call_in_event() {
        let src = "proc helper {} { return 1 }\nwhen HTTP_REQUEST { helper }";
        assert!(has_code(src, "f5-irules", "IRULE5005"));
    }

    #[test]
    fn irule5005_fires_for_direct_proc_call_in_command_substitution() {
        // A direct proc call nested inside a `[…]` substitution still bypasses
        // `call`, so IRULE5005 must reach it via the nested-segment walk.
        let src = "proc helper {} { return 1 }\nwhen HTTP_REQUEST { set x [helper] }";
        assert!(has_code(src, "f5-irules", "IRULE5005"));
    }

    #[test]
    fn irule5005_quiet_with_call_prefix() {
        let src = "proc helper {} { return 1 }\nwhen HTTP_REQUEST { call helper }";
        assert!(!has_code(src, "f5-irules", "IRULE5005"));
    }

    #[test]
    fn irule5005_quiet_outside_event_context() {
        // Top-level direct call, no `when` block — not an iRules-event call.
        let src = "proc helper {} { return 1 }\nhelper";
        assert!(!has_code(src, "f5-irules", "IRULE5005"));
    }

    #[test]
    fn irule5005_quiet_in_plain_tcl_dialect() {
        let src = "proc helper {} { return 1 }\nwhen HTTP_REQUEST { helper }";
        assert!(!has_code(src, "tcl", "IRULE5005"));
    }

    #[test]
    fn irule5005_carries_call_rewrite_fix() {
        let src = "proc helper {} { return 1 }\nwhen HTTP_REQUEST { helper x y }";
        let mut a = Analyser::new();
        let res = a.analyse(src, "f5-irules");
        let d = res
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::Irule5005)
            .expect("IRULE5005 expected");
        assert!(d.message.contains("call helper x y"));
        assert_eq!(d.fixes.len(), 1);
        assert_eq!(d.fixes[0].new_text, "call helper");
    }

    /// A math-function call in an expression resolves to the
    /// `::tcl::mathfunc::<name>` command it dispatches to: the written head is
    /// the bare tail, the resolved / settled name is the mathfunc command, and
    /// the span covers just the function token so a rename rewrites the tail.
    #[test]
    fn expr_function_call_records_a_mathfunc_invocation() {
        let src = "proc ::tcl::mathfunc::myfunc {x} { return $x }\nexpr {myfunc($a) + 1}\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let inv = r
            .command_invocations
            .iter()
            .find(|i| {
                i.name == "myfunc"
                    && i.resolved_qualified_name.as_deref() == Some("::tcl::mathfunc::myfunc")
            })
            .expect("mathfunc invocation recorded");
        assert_eq!(
            &src[inv.range.start() as usize..inv.range.end() as usize],
            "myfunc",
            "span should cover the function-name token only",
        );
        assert_eq!(inv.argc, Some(1), "one argument expression");
        assert!(
            inv.resolution_candidates
                .iter()
                .any(|c| c == "::tcl::mathfunc::myfunc"),
            "settled candidates should include the mathfunc command: {:?}",
            inv.resolution_candidates,
        );
    }

    /// A math function used before its introducing release is W002 (the
    /// `::tcl::mathfunc::<name>` command does not exist in the older core):
    /// `min` is 8.5+, the `is*` classification family is 9.0+.
    #[test]
    fn expr_function_before_its_release_is_disabled_in_dialect() {
        let flags = |src: &str, dialect: &str| {
            let mut a = Analyser::new();
            a.analyse(src, dialect)
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W002)
        };
        assert!(flags("expr {min(1, 2)}\n", "tcl8.4"), "min() is 8.5+");
        assert!(
            !flags("expr {min(1, 2)}\n", "tcl8.6"),
            "min() exists in 8.6"
        );
        assert!(flags("expr {isinf(1.0)}\n", "tcl8.6"), "isinf() is 9.0+");
        assert!(
            !flags("expr {isinf(1.0)}\n", "tcl9.0"),
            "isinf() exists in 9.0"
        );
        // An 8.4-era function is fine everywhere.
        assert!(!flags("expr {abs(-1)}\n", "tcl8.4"), "abs() is 8.4");
    }

    /// A nested function call (`sqrt(abs($x))`) records both the outer and the
    /// inner function as mathfunc invocations.
    #[test]
    fn expr_nested_function_calls_are_both_recorded() {
        let src = "expr {sqrt(abs($x))}\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        for f in ["sqrt", "abs"] {
            assert!(
                r.command_invocations.iter().any(|i| {
                    i.name == f
                        && i.resolved_qualified_name.as_deref()
                            == Some(&format!("::tcl::mathfunc::{f}"))
                }),
                "{f} should be recorded: {:?}",
                r.command_invocations
                    .iter()
                    .map(|i| &i.name)
                    .collect::<Vec<_>>(),
            );
        }
    }

    /// An unrelated ordinary `proc sin` sharing a math function's bare tail
    /// name must never hijack `expr {sin(...)}` — the two live in entirely
    /// separate command tables in real Tcl (confirmed by the VM's own
    /// `tcl::mathfunc::*` dispatch, which never consults an ordinary
    /// top-level command). Before the two-candidate resettlement rule
    /// below, `finalise_invocation_resolutions`'s generic one-hop
    /// `{ns}::{name}` suffix-strip misparsed the mathfunc-qualified
    /// `::tcl::mathfunc::sin` as if `::tcl::mathfunc` were the *calling*
    /// namespace, and — once `sin` was `known` only as the unrelated
    /// global proc — silently rewrote the resolved name to `::sin`.
    #[test]
    fn expr_function_call_ignores_unrelated_same_named_global_proc() {
        let src = "proc sin {x} { return bogus }\nset y [expr {sin(1.0)}]\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let inv = r
            .command_invocations
            .iter()
            .find(|i| i.name == "sin" && i.argc == Some(1))
            .expect("mathfunc invocation recorded");
        assert_eq!(
            inv.resolved_qualified_name.as_deref(),
            Some("::tcl::mathfunc::sin"),
            "the unrelated proc sin must not hijack expr's sin(...): {inv:?}",
        );
    }

    /// TIP 232: a namespace-local `proc ::ns::tcl::mathfunc::f` shadows the
    /// global `::tcl::mathfunc::f` for a call made from inside `::ns` —
    /// confirmed real Tcl behaviour by the VM's
    /// `namespace_local_mathfunc_shadows_global_in_expr`
    /// (`tcl-vm/tests/tricky_resolution_e2e.rs`). The analyser must settle
    /// the call to the local override, not the fixed global form.
    #[test]
    fn expr_function_call_resolves_namespace_local_mathfunc_override() {
        let src = "namespace eval ::nsa::tcl::mathfunc {}\n\
                    proc ::nsa::tcl::mathfunc::pf {x} { return 20 }\n\
                    namespace eval ::nsa {\n    proc caller {} { return [expr {pf(1)}] }\n}\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let inv = r
            .command_invocations
            .iter()
            .find(|i| i.name == "pf")
            .expect("mathfunc invocation recorded");
        assert_eq!(
            inv.resolved_qualified_name.as_deref(),
            Some("::nsa::tcl::mathfunc::pf"),
            "pf(1) inside ::nsa must resolve to the local override: {inv:?}",
        );
        assert_eq!(
            inv.resolution_candidates,
            vec!["::nsa::tcl::mathfunc::pf", "::tcl::mathfunc::pf"],
            "local-first, then global, matching the VM's own search order",
        );
    }

    /// The companion of the override case above: no local override exists at
    /// `::nsa::tcl::mathfunc::pf`, only a *global* user-defined one — the
    /// call still finds it via the two-candidate rule's fallback step.
    #[test]
    fn expr_function_call_falls_back_to_global_user_override_from_a_namespace() {
        let src = "proc ::tcl::mathfunc::pf {x} { return 10 }\n\
                    namespace eval ::nsa {\n    proc caller {} { return [expr {pf(1)}] }\n}\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let inv = r
            .command_invocations
            .iter()
            .find(|i| i.name == "pf")
            .expect("mathfunc invocation recorded");
        assert_eq!(
            inv.resolved_qualified_name.as_deref(),
            Some("::tcl::mathfunc::pf"),
            "no local override at ::nsa -- must fall back to the global proc: {inv:?}",
        );
    }

    /// The everyday case exercised via the new namespace-aware resolution
    /// path: a built-in (`sin`), called from inside a namespace with no
    /// override anywhere, must still settle to the global built-in slot —
    /// the fix to the collision/shadowing bugs above must not regress the
    /// common no-namespace, no-override call this project's issue #968
    /// fix already covers at the diagnostic layer.
    #[test]
    fn expr_function_call_resolves_builtin_from_inside_a_namespace() {
        let src = "namespace eval ::nsa {\n    proc caller {} { return [expr {sin(1.0)}] }\n}\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let inv = r
            .command_invocations
            .iter()
            .find(|i| i.name == "sin")
            .expect("mathfunc invocation recorded");
        assert_eq!(
            inv.resolved_qualified_name.as_deref(),
            Some("::tcl::mathfunc::sin"),
            "a built-in with no override anywhere must settle globally: {inv:?}",
        );
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
            "must still draw no W123: {:?}",
            r.diagnostics,
        );
    }

    /// TIP 232 math functions are ordinary commands, so `namespace path`
    /// applies to their resolution exactly as it does to any other command —
    /// confirmed against the VM's own `resolve_command_fqn`, which routes
    /// every lookup (mathfunc calls included) through the same `ns_paths`-
    /// aware resolver. Neither the caller's own namespace nor the global
    /// slot defines `triple`; only the `namespace path` entry does.
    #[test]
    fn expr_function_call_honours_namespace_path() {
        let src = "namespace eval ::libns::tcl::mathfunc {}\n\
                    proc ::libns::tcl::mathfunc::triple {x} { return [expr {$x * 3}] }\n\
                    namespace eval ::consumer {\n    \
                        namespace path ::libns\n    \
                        proc caller {} { return [expr {triple(2)}] }\n\
                    }\n";
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let inv = r
            .command_invocations
            .iter()
            .find(|i| i.name == "triple")
            .expect("mathfunc invocation recorded");
        assert_eq!(
            inv.resolved_qualified_name.as_deref(),
            Some("::libns::tcl::mathfunc::triple"),
            "must settle via the namespace path entry: {inv:?}",
        );
        assert_eq!(
            inv.resolution_candidates,
            vec![
                "::consumer::tcl::mathfunc::triple",
                "::libns::tcl::mathfunc::triple",
                "::tcl::mathfunc::triple",
            ],
            "current namespace, then the path entry, then global",
        );
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W123),
            "must draw no W123: {:?}",
            r.diagnostics,
        );
    }
}
