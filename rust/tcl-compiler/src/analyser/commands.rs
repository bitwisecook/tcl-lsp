//! Central command dispatch — Rust port of the core handler-call
//! portion of ``_AnalyserCommandsMixin._process_command`` in
//! ``core/analysis/_analyser/_commands.py:118-540``.
//!
//! Walks one segmented Tcl command and routes it through the
//! per-command handlers landed in C41b1-C41b6. The Python source
//! is ~466 LOC because it interleaves several concerns:
//!
//! 1. **Handler dispatch** (this strip — C41b7).
//! 2. Var/cmd-as-command site recording for W307 — deferred to
//!    **C41d3**.
//! 3. W125 (orphaned control-flow keyword) — deferred to
//!    **C41d5** (`_diag_branches.py` orchestration).
//! 4. IRULE5005 (direct iRules-proc call without ``call``) —
//!    deferred to **C41d6**.
//! 5. Arity checks — already live in
//!    ``compiler_checks::arity_checks``.
//! 6. Sub-command resolution / unresolved-command tracking —
//!    deferred to **C41d4**.
//!
//! C41b7 lands the core dispatch only — when later strips need
//! to interleave additional concerns, they extend
//! [`Analyser::process_command`] in place rather than each
//! adding a parallel walker. The dispatch shape is a series of
//! ``if let true = self.handle_xxx(...) { return }`` calls so
//! extending it remains a one-liner.

use tcl_lexer::{Lexer, LexerConfig, SourceMap, Span, Token, TokenType};
use tcl_registry::{ArgRole, CommandRegistry};

use crate::parsing::syntax::descend::{descend_command, descend_token};
use crate::parsing::syntax::segment::segments_from_tree;
use crate::segmenter::SegmentedCommand;

use super::state::Analyser;

impl Analyser {
    /// Re-segment a body script and dispatch each command at
    /// `scope_path`.
    ///
    /// Mirrors the post-segmentation portion of
    /// `_analyse_body_inner` in
    /// `core/analysis/_analyser/_core.py:438-524` — the analyser-
    /// side of body recursion. Used by every body-walking handler
    /// (`handle_proc_command`, `handle_switch_command`,
    /// `handle_try_command`, `handle_catch_command`, etc.).
    ///
    /// Body recursion does **not** use the segmenter's re-segmentation
    /// recovery (Seg2) — that splits a runaway top-level command and only
    /// fires at the top level. The per-command syntax *detectors* (E100 /
    /// E102 stray closers, E201 unterminated `[`, E202 unterminated `"`,
    /// E203 unterminated `{`) do run on every body, matching Python's
    /// `_analyse_body_inner`, whose `segment_with_recovery(source,
    /// body_token)` runs the same detectors over body content.
    /// Dynamic bodies (`$body`, `[gen]`) are skipped because they
    /// can't be statically re-segmented.
    ///
    /// `body_depth` is bumped for the duration of the walk so
    /// top-level-only command checks (deferred to **C41d**) can
    /// distinguish nested invocations.
    ///
    /// **Deferred concerns** (each gets its own future strip):
    /// var-read recording for `VAR` tokens, `CMD`-substitution
    /// recursion, preceding-comment harvesting, and the
    /// recovery hooks (`recover_stray_close_bracket`,
    /// `recover_missing_open_brace`).  This helper covers the
    /// minimal subset C41c needs.
    pub(super) fn analyse_body(&mut self, body_text: &str, body_tok: Token, scope_path: &[usize]) {
        if body_tok.kind != TokenType::Str {
            return;
        }
        self.body_depth += 1;
        let base_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
        // Absolute byte offset at which this body region ends. The E202 /
        // E203 detectors test "reaches end of region" against this, not the
        // whole document — the body's tokens are absolute spans into
        // `self.source`, but a runaway `"` / `{` only swallows to the body's
        // own end. Mirrors Python's `base_offset + len(source)` arithmetic.
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
                // GAP-A1: an unterminated `"` / `{` emits the precise E202 /
                // E203 (with a closing-delimiter fix) ahead of the generic
                // E200, mirroring the top-level `walk_commands_top_level`
                // branch.  **C41e5.** Stolen-close-brace detection ⇒ E103
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
            // GAP-A6 follow-up: run the E100 (stray `]`) / E102
            // (stray `}`) token checks on every analysed body, not
            // just the top level — mirrors Python's
            // ``_UNIVERSAL_CHECKS`` (`check_unmatched_close_bracket`
            // / `check_unmatched_close_brace`) running on every
            // command.  Run on the *original* token stream before
            // ``recover_stray_close_bracket`` repairs the clone,
            // matching the top-level loop's ordering.  Token spans
            // are absolute into the full document, so the full
            // ``self.source`` is the right slice base.
            let stray = super::syntax_checks::stray_closer_diagnostics(
                cmd_ref,
                &self.source,
                self.registry.as_ref(),
            );
            self.result.diagnostics.extend(stray);
            // E201 (unterminated `[`) inside a body — `proc p {} { set y
            // [foo }`.  The CST auto-closes the bracket so the command isn't
            // flagged `is_partial`, but the source carries no real `]`, so
            // it would otherwise go unreported.  Mirror the top-level
            // `emit_syntax_recovery_diagnostics` E201 detector (the
            // top-level ghost-recovery doesn't reach into body scripts).
            let e201 = super::syntax_checks::unterminated_bracket_diagnostics(
                cmd_ref,
                &self.source,
                self.registry.as_ref(),
            );
            self.result.diagnostics.extend(e201);
            // E202 (unterminated `"`) / E203 (unterminated `{`) inside a
            // body — `proc p {} { set x "\n puts hi }`.  The brace word the
            // body sits in is balanced, so the body re-segments cleanly and
            // the run-away quote/brace is a non-partial command whose token
            // reaches the body's end.  Mirror the top-level
            // `emit_syntax_recovery_diagnostics` E202/E203 detector, which
            // the top-level ghost-recovery doesn't reach into body scripts —
            // matching Python's `_analyse_body_inner`, whose
            // `segment_with_recovery` runs the same detectors on every body.
            self.emit_unterminated_delimiter_diagnostics(cmd_ref, region_end);
            let mut cmd = cmd_ref.clone();
            // **C41e4.** Repair stray ``]`` (missing ``[``) so
            // downstream handlers see the intended argv shape
            // before dispatch.
            self.recover_stray_close_bracket(&mut cmd);
            // **C41e5.** Splice orphaned switch case pairs
            // when ``{`` was forgotten.  The returned count is
            // added to ``cmd_idx`` so we skip past the consumed
            // orphans.
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
            // `S-document-highlight-rich` / `S-references-rich`
            // follow-up: record every `$var` substitution in
            // arg positions so `VarDef.references` carries the
            // read spans the LSP providers consume.
            self.record_arg_var_reads(&cmd, scope_path);
            cmd_idx += 1 + consumed;
        }
        self.body_depth -= 1;
    }

    /// Process a single segmented command.
    ///
    /// Mirrors the **handler-dispatch** subset of
    /// ``_process_command`` in
    /// ``core/analysis/_analyser/_commands.py:118-540``. Walks
    /// `args` against every handler in C41b1-C41b6 and stops at
    /// the first match. Non-matching commands fall through
    /// silently — they're either unknown (W123 emitter handles
    /// reporting in **C41d4**) or registry-known commands that
    /// don't need analyser-side intervention (the IR pass does
    /// the heavy lifting).
    ///
    /// `argv_texts` and `arg_tokens` parallel each other:
    /// `argv_texts[0]` is the command name, `argv_texts[1..]`
    /// the arguments. `arg_tokens[0]` is the command-name token.
    /// `single_token_word` is parallel to argv and indicates
    /// whether each word is a single atomic token (used by
    /// ``handle_set_command`` for the const-string heuristic).
    ///
    /// Deferred concerns (each gets its own future strip — see
    /// the module docstring): var-as-command site recording,
    /// W125 / IRULE5005 emission, sub-command resolution,
    /// command-invocation recording with resolved-qname annotation.
    /// Simple-command arity (E002 / E003) lands here via
    /// [`Self::emit_arity_diagnostics`] (SYNC-MAY21-3); the
    /// candidates are flushed post-walk by
    /// [`Self::flush_arity_diagnostics`].
    #[allow(clippy::too_many_lines)]
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

        // **C41d4.** Record this invocation so the post-walk
        // ``emit_unresolved_command_diagnostics`` (W123) can iterate
        // every command head the analyser visited.  Mirrors the
        // matching ``self.result.command_invocations.append(...)``
        // call in ``_AnalyserCommandsMixin._process_command``
        // (``core/analysis/_analyser/_commands.py``).  ``inv.range``
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
            let resolved = self.resolve_command_qualified_name(cmd_name);
            self.result.command_invocations.push(
                crate::signature_scan::types::SignatureCommandInvocation {
                    name: cmd_name.to_string(),
                    range: cmd_tok.span,
                    resolved_qualified_name: Some(resolved),
                },
            );

            // iRules ``call PROC ARG...`` — record an additional
            // ``CommandInvocation`` for the target proc so that
            // references, rename, and call-hierarchy see through the
            // indirection.  Mirrors
            // ``_AnalyserCommandsMixin._process_command`` line 231 in
            // ``core/analysis/_analyser/_commands.py``.
            if cmd_name == "call"
                && self.dialect == "f5-irules"
                && let (Some(target_name), Some(target_tok)) =
                    (args.first(), arg_tokens_in.get(1).copied())
            {
                let resolved = self.resolve_command_qualified_name(target_name);
                self.result.command_invocations.push(
                    crate::signature_scan::types::SignatureCommandInvocation {
                        name: target_name.clone(),
                        range: target_tok.span,
                        resolved_qualified_name: Some(resolved),
                    },
                );
            }

            // Walk every argument's source slice for ``[cmd ...]``
            // substitutions and record each nested head as its own
            // ``CommandInvocation``.  Mirrors ``_iter_nested_invocations``
            // in ``_AnalyserCommandsMixin._record_command_invocation``
            // (Python).
            self.record_nested_invocations_from_args(cmd_name, args, arg_tokens_in);

            // Run the per-command syntactic checks on commands nested inside
            // ``[…]`` substitutions — the main walk never descends a
            // substitution (it treats `[cmd …]` as a value), so a command
            // like `set fh [open "|$cmd" r]` or `set x [string index abc 99]`
            // would otherwise escape the security / bounds / arity / style
            // families entirely.  Mirrors main's ``_recurse_nested_commands``
            // re-running ``run_all_checks`` on each descended substitution
            // command.
            self.run_nested_command_diagnostics(arg_tokens_in, scope_path);

            // Run the per-command syntactic + EXPR checks on commands nested
            // inside a ``[…]`` substitution of a *braced expression*
            // argument (`if { [matchclass …] }`, `while { [done $x] }`).
            // The bare-`Cmd` walk above never enters a braced `Str` expr
            // arg, so those substitution commands would otherwise escape
            // every per-command check (IRULE2001/2002, W100, …).  Mirrors
            // main's ``_recurse_expression_subcommands``.
            self.run_nested_expr_diagnostics(cmd_name, args, arg_tokens, scope_path);

            // **C41d3.** Record variable-as-command and
            // command-substitution-as-command call sites so the
            // post-walk W307 / W308 emitters can resolve them.
            // Mirrors the inline recording in
            // ``_AnalyserCommandsMixin._process_command``
            // (``_commands.py:182-198``).
            self.record_var_or_cmd_command_site(cmd_tok, args, scope_path);

            // Record TclOO instance creation (`set v [Cls new]`,
            // `Cls create inst`) so the LSP providers can resolve
            // ``$v method`` / ``inst method`` call sites to the
            // object's class.
            self.record_instance_creation(cmd_name, args);

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
            self.dispatch_expr_arguments(cmd_name, args, arg_tokens);

            // Dispatch-site diagnostic emitters (W302 / W001 / E004 / W101
            // / W304 / W004 / E002-E003).  Extracted from this function so
            // it stays within the line budget; see the method for the
            // per-code rationale and ordering.  Run before the
            // early-returning handlers so option-bearing / body-owning
            // commands still get checked.
            self.emit_dispatch_site_diagnostics(
                cmd_name,
                args,
                arg_tokens,
                arg_single,
                arg_expand_in,
                cmd_tok,
                scope_path,
            );
        } // end `if !self.structure_only`

        // Handler-by-handler dispatch. Each returning-bool
        // handler is consulted in turn; first match wins. The
        // void-returning handlers run unconditionally (their
        // own internal cmd-name guard rejects mismatches).

        // proc — registers the proc record + scope.
        if self.handle_proc_command(cmd_name, args, arg_tokens, scope_path) {
            return;
        }

        // oo::class create / oo::define — class records + body
        // walk (C41e1 / C41e2 fill in the body walks).
        if self.handle_oo_class_command(cmd_name, args, arg_tokens, scope_path) {
            return;
        }
        if self.handle_oo_define_command(cmd_name, args, arg_tokens, scope_path) {
            return;
        }

        // namespace eval — opens a namespace child scope.
        if self.handle_namespace_eval_command(cmd_name, args, arg_tokens, scope_path) {
            return;
        }

        // uplevel #0 { body } — opens a global-frame child scope so the
        // body's locals don't leak into the enclosing proc's variable
        // set.  Only the `#0` form is handled here; other levels fall
        // through to the generic body recursion below.
        if self.handle_uplevel_command(cmd_name, args, arg_tokens, scope_path) {
            return;
        }

        // foreach / for / switch / catch / try — entry shims;
        // body recursion lands in C41f1.
        if self.handle_foreach_command(cmd_name, args, arg_tokens, scope_path) {
            return;
        }
        if self.handle_for_command(cmd_name, args, arg_tokens, scope_path) {
            return;
        }
        if self.handle_switch_command(cmd_name, args, arg_tokens, scope_path) {
            return;
        }
        if self.handle_catch_command(cmd_name, args, arg_tokens, scope_path) {
            return;
        }
        if self.handle_try_command(cmd_name, args, arg_tokens, scope_path) {
            return;
        }

        // Variable-mutating handlers. These are void-returning
        // and silently no-op if the cmd_name doesn't match —
        // safe to call sequentially.
        self.handle_set_command(cmd_name, args, arg_tokens, arg_single, scope_path);
        self.handle_var_declaration_command(cmd_name, args, arg_tokens, scope_path);
        self.handle_incr_command(cmd_name, args, arg_tokens, scope_path);
        // Local-alias / loop-var bindings: `upvar`, `namespace upvar`, and
        // `dict for/update/with` introduce names visible to completion / hover.
        self.handle_upvar_command(cmd_name, args, arg_tokens, scope_path);
        self.handle_namespace_upvar_command(cmd_name, args, arg_tokens, scope_path);
        self.handle_dict_var_command(cmd_name, args, arg_tokens, scope_path);

        // Side-effect-only handlers. Same idempotent pattern.
        self.handle_namespace_ensemble(cmd_name, args, scope_path);
        self.handle_interp_alias(cmd_name, args);
        self.handle_oo_objdefine(cmd_name, args);
        self.handle_package_command(cmd_name, cmd_tok, args, arg_tokens);
        self.handle_source_command(cmd_name, args, arg_tokens);
        self.handle_namespace_import_command(cmd_name, args, arg_tokens, scope_path);
        self.handle_tcllib_import_wrapper(cmd_name, cmd_tok, args, scope_path);
        self.handle_auto_path_command(cmd_name, args, arg_tokens);
        self.handle_regex_pattern_capture(cmd_name, args, arg_tokens, scope_path);

        // ``load`` / ``rename`` flip ``has_dynamic_providers``.
        // ``load`` brings a shared library's commands into the
        // interpreter at runtime; ``rename`` can introduce new
        // command names dynamically.  Both make static W123
        // unknown-command analysis unreliable, so the flag
        // suppresses those diagnostics on the document.  Mirrors
        // Python's ``_commands.py`` behaviour.
        if matches!(cmd_name, "load" | "rename") {
            self.result.has_dynamic_providers = true;
        }

        // Generic body recursion via the command registry's
        // `ArgRole::Body`.  Mirrors the `iter_body_arguments` loop
        // in `_AnalyserCommandsMixin._process_command` (Python).
        // Picks up `if` / `while` / `when` / `eval` / `uplevel`
        // / `subst` / etc. — every command whose registry spec
        // marks an argument index as `BODY`.  The dedicated
        // `handle_*_command` calls above already returned early
        // for the commands they own (proc, oo::class, oo::define,
        // namespace eval, foreach, for, switch, catch, try), so
        // this loop only fires for the rest.
        //
        // For `when EVENT { body }` the iRules dialect spec
        // marks arg 1 as BODY; set `current_event` for the body
        // walk so race-detection diagnostics see the event
        // name, mirroring the Python behaviour.
        self.dispatch_body_arguments(cmd_name, args, arg_tokens, arg_single, scope_path);
    }

    /// Dispatch-site diagnostic emitters, run from
    /// [`Self::process_command`] before the early-returning handlers so
    /// option-bearing / body-owning commands still get checked.
    ///
    /// - **W302** (`catch` without a result variable) — `IRCatch` arm
    ///   of `_check_statement` (`compiler_checks.py:491-504`); fires
    ///   before the early-returning `handle_catch_command`.
    /// - **W001** (unknown subcommand on a `SubcommandSig` command) —
    ///   `SubcommandSig` branch of `_check_arity`
    ///   (`compiler_checks.py:580-643`); before
    ///   `handle_namespace_eval_command` so `namespace foo` is flagged.
    /// - **E004** (malformed `if`) — `IRBarrier` arm of
    ///   `_check_statement` (`compiler_checks.py:506-525`).
    /// - **W101** (`eval` with substituted args) —
    ///   `check_eval_string_concat` (`checks/_security.py:19-73`);
    ///   before body-walk dispatch so the `ArgRole::Body` recursion
    ///   into the `eval` body still runs.
    /// - **W304** (missing `--` option terminator) —
    ///   `check_missing_option_terminator` (`checks/_style.py:506-679`),
    ///   driven by the registry's option-terminator profile.
    /// - **W004** (option not available in the active dialect,
    ///   SYNC-MAY19-W003-W004) — `check_dialect_invalid_option`
    ///   (`checks/_domain.py`, PR #433).
    /// - **E002 / E003** (arity, SYNC-MAY21-3) — collected here and
    ///   flushed post-walk by [`Self::flush_arity_diagnostics`].
    #[allow(clippy::too_many_arguments)]
    fn emit_dispatch_site_diagnostics(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        arg_single: &[bool],
        arg_expand_in: &[bool],
        cmd_tok: Token,
        scope_path: &[usize],
    ) {
        if cmd_name == "catch" {
            self.emit_w302_catch_no_result_var(args, cmd_tok, arg_tokens, arg_single);
        }
        self.emit_w001_unknown_subcommand(cmd_name, args, cmd_tok, arg_tokens);
        self.emit_w002_disabled_command(cmd_name, cmd_tok);
        if cmd_name == "if" {
            self.emit_e004_malformed_if(args, cmd_tok, arg_tokens);
        }
        self.emit_w101_eval_string_concat(cmd_name, args, arg_tokens, arg_single);
        // W102 / W103 / W300 / W301 / W309 / W312 security-injection
        // checks (GAP-A2), ported from `core/analysis/checks/_security.py`.
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
        // IRULE2002: deprecated iRules command (f5-irules only).
        self.emit_irule2002_deprecated_command(cmd_name, cmd_tok);
        // IRULE2001: deprecated `matchclass` (f5-irules only).  Python
        // fires this alongside IRULE2002 at the same command-head span.
        self.emit_irule2001_matchclass(cmd_name, cmd_tok);
        // IRULE1003 / 1004 / 2101 / 4001 / 4003 / 5001 / 6001 —
        // analyser-level iRules event-context checks (f5-irules only).
        self.emit_irules_event_checks(cmd_name, args, arg_tokens, cmd_tok);
        self.emit_w212_name_vs_value(cmd_name, args, arg_tokens);
        self.emit_w104_append_list(cmd_name, args, arg_tokens);
        self.emit_w106_unbraced_switch_body(cmd_name, args, arg_tokens);
        self.emit_w311_encoding_mismatch(cmd_name, args, arg_tokens);
        self.emit_w200_binary_format_modifiers(cmd_name, args, arg_tokens);
        self.emit_w121_invalid_subnet_mask(args, arg_tokens);
        self.emit_w108_non_ascii(arg_tokens);
        // W240 / W241 loop-termination + W230 / W232 index-bounds (GAP-A4).
        let loop_diags =
            super::bounds_checks::loop_termination_diagnostics(cmd_name, args, arg_tokens);
        self.result.diagnostics.extend(loop_diags);
        let idx_diags = super::bounds_checks::list_index_diagnostics(cmd_name, args, arg_tokens);
        self.result.diagnostics.extend(idx_diags);
        let lset_diags =
            super::bounds_checks::lset_index_diagnostics(cmd_name, args, arg_tokens, &self.source);
        self.result.diagnostics.extend(lset_diags);
        let str_diags = super::bounds_checks::string_index_diagnostics(cmd_name, args, arg_tokens);
        self.result.diagnostics.extend(str_diags);
        self.emit_w127_closed_value_args(cmd_name, args, arg_tokens, cmd_tok);
        self.emit_w304_missing_option_terminator(cmd_name, args, cmd_tok, arg_tokens);
        self.emit_w004_dialect_invalid_option(cmd_name, args, arg_tokens);
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
    /// `ArgRole::Expr`.  Mirrors the EXPR slice of
    /// `iter_body_arguments` plus the explicit per-check
    /// dispatchers in `core/analysis/checks/_style.py`.
    /// Currently invokes the W110 emitter on each EXPR-role
    /// argument.  For `expr`, multi-arg invocations are joined
    /// with spaces before the W110 walk — matches Python's
    /// `expr_text = " ".join(args)` special-case.
    fn dispatch_expr_arguments(&mut self, cmd_name: &str, args: &[String], arg_tokens: &[Token]) {
        let Some(registry) = self.registry.as_ref() else {
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

        // W100 (GAP-A8): unbraced expression argument. Runs for every
        // EXPR-role form, including the `expr 1 + 2` multi-word case
        // handled by the early return below.
        self.emit_w100_unbraced_expr(cmd_name, args, arg_tokens);

        // Special-case ``expr ...``: when the user wrote multiple
        // arguments (``expr $a == "x"`` instead of the more common
        // ``expr {$a eq "x"}``), Python anchors W110 / W003 at the full
        // argument token range and parses ``" ".join(args)`` — the
        // *substituted* word values, with quote delimiters already
        // stripped by Tcl's word splitting.  So ``expr $a == "x"`` parses
        // as ``$a == x`` where ``x`` is a bareword, not an ``ExprString``,
        // and W110 (string ``==``) does NOT fire — matching what `expr`
        // actually receives at runtime.  (The earlier source-slice text
        // kept the quotes and over-fired W110 vs Python.)
        if cmd_name == "expr" && args.len() > 1 && !arg_tokens.is_empty() {
            let span = tcl_lexer::Span::new(
                arg_tokens[0].span.start(),
                arg_tokens[arg_tokens.len() - 1].span.end(),
            );
            let expr_text = args.join(" ");
            self.emit_w110_string_eq_ne(&expr_text, span);
            self.emit_w003_dialect_invalid_expr_operator(&expr_text, span);
            return;
        }

        for idx in indices {
            if let (Some(text), Some(tok)) = (args.get(idx), arg_tokens.get(idx)) {
                self.emit_w110_string_eq_ne(text, tok.span);
                self.emit_w003_dialect_invalid_expr_operator(text, tok.span);
                self.emit_w114_redundant_nested_expr(text, tok.span);
            }
        }
    }

    /// Generic body recursion via the command registry's
    /// `ArgRole::Body`.  Mirrors the `iter_body_arguments` loop
    /// in `_AnalyserCommandsMixin._process_command` (Python).
    /// Picks up `if` / `while` / `when` / `eval` / `uplevel`
    /// / `subst` / etc. — every command whose registry spec
    /// marks an argument index as `BODY`.  Sets
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
        let Some(registry) = self.registry.as_ref() else {
            return;
        };
        let body_args: Vec<&str> = args.iter().map(String::as_str).collect();
        // Body-role resolution stays dialect-scoped *deliberately*: a command
        // that owns a body only in another dialect (e.g. the iRules-only
        // `when`) is, under a plain-tcl dialect, an unknown user command whose
        // braced `{...}` is an ordinary string argument — not a script. So we
        // do NOT recurse into it (and do not fire W123/W002 on its contents).
        // Python applies the iRules `when` BODY role even under tcl8.6, which
        // leaks iRules semantics into non-iRules analysis; that divergence is
        // intentional (Rust is the more-correct side). Analyse iRules under
        // the f5-irules dialect, where `when` is a real body-owning command.
        let body_indices = registry.arg_indices_for_role(
            cmd_name,
            &body_args,
            tcl_registry::arg_role::ArgRole::Body,
        );
        if body_indices.is_empty() {
            return;
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
                self.emit_w105_unbraced_body(cmd_name, body_text, body_tok, is_single_token);
                self.analyse_body(body_text, body_tok, scope_path);
            }
        }
        if is_conditional {
            self.conditional_depth -= 1;
        }
        if cmd_name == "when" {
            self.current_event = prev_event;
        }
    }

    /// Walk every argument's source slice for ``[cmd ...]``
    /// substitutions and record each nested head as its own
    /// ``CommandInvocation``.  Extracted from
    /// [`Self::process_command`] for readability — without this,
    /// calls embedded inside argument expressions
    /// (``set x [helper $foo]``, ``puts "got [count $items]"``,
    /// ``if { [HTTP::uri] eq "/foo" }``) aren't tracked, which
    /// breaks workspace usage counts, find-references, rename,
    /// and call-hierarchy.  Mirrors ``_iter_nested_invocations``
    /// in ``_AnalyserCommandsMixin._record_command_invocation``
    /// (Python).
    fn record_nested_invocations_from_args(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens_in: &[Token],
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
        // A command whose *name* is itself a substitution (`[x] hi`) — main's
        // `_recurse_nested_commands` iterates every token including the head,
        // so descend a `Cmd` head too (the head's *name* is recorded
        // separately by `process_command`; this records what it substitutes).
        if let Some(head) = arg_tokens_in.first()
            && head.kind == TokenType::Cmd
        {
            self.record_invocations_from_cmd_token(*head);
        }
        for (i, arg_tok) in arg_tokens_in.iter().enumerate().skip(1) {
            let arg_start = arg_tok.span.start();
            let arg_end = arg_tok.span.end() as usize;
            let src_len = self.source.len();
            if arg_start as usize >= src_len || arg_end > src_len {
                continue;
            }
            if arg_tok.kind == TokenType::Cmd {
                self.record_invocations_from_cmd_token(*arg_tok);
            } else if arg_tok.kind == TokenType::Str {
                // Braced word: scan its `[...]` substitutions only when it
                // is an expression argument (substitutions are then active);
                // a braced data word stays opaque (mirrors main, which never
                // walks a non-expr braced word as a script).
                if expr_indices.contains(&(i - 1)) {
                    self.record_invocations_from_expr_token(*arg_tok);
                }
            } else {
                // `Esc` (bareword / quoted): substitutions are active, so
                // scan.  Clone the slice into an owned ``String`` so the
                // helper can take ``&mut self`` without conflicting with the
                // source borrow.
                let arg_src = self.source[arg_start as usize..arg_end].to_string();
                self.record_invocations_from_word_token(*arg_tok, &arg_src, arg_start);
            }
        }
    }

    /// Inner: ``Cmd`` (``[…]``) substitution tokens.  Descend the
    /// substitution into a child CST ([`descend_token`]) and record
    /// *every* inner command's bareword head, recursing into nested
    /// ``[...]``.  Mirrors main's ``_recurse_nested_commands`` (segment
    /// the substitution, process each command).
    ///
    /// The previous flat [`first_command_head`] scan recorded only the
    /// *first* head of each ``[...]``, so ``;``- / newline-separated
    /// commands were dropped (`[foo; bar]` → only `foo`); the CST
    /// descent finds them all (CST-CONSUMERS strip 1).  This is the
    /// first production caller of the landed-but-unused `descend_token`.
    fn record_invocations_from_cmd_token(&mut self, arg_tok: Token) {
        let config = self.lexer_config();
        // Collect the inner heads first (this borrows `self.source`
        // through the `SourceMap`); resolve + push afterwards so the
        // immutable source borrow has ended.
        let heads = {
            let sm = SourceMap::new(&self.source);
            let mut heads: Vec<(String, Span)> = Vec::new();
            // `arg_tok` is the *merged* argv token.  For a compound word
            // whose first fragment is a `[…]` substitution (`[foo]bar`,
            // `[foo]$x`, `[foo]bar[baz]`), `segments_from_tree` widens the
            // span from the first fragment's start to the *last* fragment's
            // end.  Descending that merged span would re-lex the trailing
            // literal as a script and record a bogus head (`[foo]bar` →
            // `foo]bar`).  Descend each `[…]` fragment instead, mirroring
            // main's walk over the unmerged token stream; a single-fragment
            // `[…]` word yields just itself.
            for frag in self.cmd_fragments(arg_tok, config) {
                collect_substitution_heads(&sm, self.registry.as_ref(), frag, config, &mut heads);
            }
            heads
        };
        self.push_collected_heads(heads);
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
    /// otherwise escape the per-command checks.  Mirrors main's
    /// ``_recurse_nested_commands`` re-running ``run_all_checks`` on each
    /// descended substitution command.
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
            let sm = SourceMap::new(&self.source);
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
                                self.registry.as_ref(),
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
                    // `run_nested_expr_diagnostics`.  Mirrors Python's
                    // `_recurse_nested_commands` descending nested `Cmd` tokens
                    // inside quoted words.
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
                                    self.registry.as_ref(),
                                    seg,
                                    config,
                                    &mut nested,
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
    /// are live commands the main walk never reaches.  Mirrors main's
    /// ``_recurse_expression_subcommands`` re-running ``run_all_checks`` on
    /// each substitution command found inside an EXPR-role argument.
    fn run_nested_expr_diagnostics(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        let expr_indices: Vec<usize> = match self.registry.as_ref() {
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
            let sm = SourceMap::new(&self.source);
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
                                self.registry.as_ref(),
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
    /// only the syntactic half had been reaching substitution commands, so
    /// an unbraced `expr` inside `[…]` (`set y [expr $a + $b]`) escaped
    /// W100.  Mirrors main feeding nested commands through the same
    /// ``run_all_checks`` entry point as top-level ones.
    fn dispatch_nested_segment(&mut self, seg: &SegmentedCommand, scope_path: &[usize]) {
        if seg.texts.is_empty() || seg.argv.is_empty() {
            return;
        }
        let cmd_name = seg.texts[0].clone();
        let cmd_tok = seg.argv[0];
        let args = seg.texts.get(1..).unwrap_or(&[]);
        let arg_tokens = seg.argv.get(1..).unwrap_or(&[]);
        let arg_single = seg.single_token_word.get(1..).unwrap_or(&[]);
        // `emit_arity_diagnostics` expects the expand array parallel to
        // the *full* argv (head at index 0), matching `process_command`.
        let arg_expand = seg.expand_word.as_deref().unwrap_or(&[]);
        self.emit_dispatch_site_diagnostics(
            &cmd_name, args, arg_tokens, arg_single, arg_expand, cmd_tok, scope_path,
        );
        self.dispatch_expr_arguments(&cmd_name, args, arg_tokens);
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
    fn record_invocations_from_expr_token(&mut self, expr_tok: Token) {
        let config = self.lexer_config();
        let heads = {
            let sm = SourceMap::new(&self.source);
            let mut heads: Vec<(String, Span)> = Vec::new();
            collect_expr_substitutions(&sm, self.registry.as_ref(), expr_tok, config, &mut heads);
            heads
        };
        self.push_collected_heads(heads);
    }

    /// Resolve each collected `(name, span)` head to a qualified name and
    /// push it as a `command_invocations` entry.
    fn push_collected_heads(&mut self, heads: Vec<(String, Span)>) {
        for (name, range) in heads {
            let resolved = self.resolve_command_qualified_name(&name);
            self.result.command_invocations.push(
                crate::signature_scan::types::SignatureCommandInvocation {
                    name,
                    range,
                    resolved_qualified_name: Some(resolved),
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
            let resolved = self.resolve_command_qualified_name(&name);
            self.result.command_invocations.push(
                crate::signature_scan::types::SignatureCommandInvocation {
                    name,
                    range: tcl_lexer::Span::new(abs_start, abs_end),
                    resolved_qualified_name: Some(resolved),
                },
            );
        }
    }

    /// Record this command as a variable-as-command (``$obj
    /// method ...``) or command-substitution-as-command
    /// (``[expr ...] args``) call site so the post-walk W307 /
    /// W308 emitters can resolve them.  Mirrors the inline
    /// recording in ``_AnalyserCommandsMixin._process_command``
    /// (``core/analysis/_analyser/_commands.py:182-198``).  OO
    /// method-context detection lands in C41e.
    fn record_var_or_cmd_command_site(
        &mut self,
        cmd_tok: Token,
        args: &[String],
        scope_path: &[usize],
    ) {
        let in_method = self.scope_path_in_method_body(scope_path);
        match cmd_tok.kind {
            TokenType::Var => {
                let sm = tcl_lexer::SourceMap::new(&self.source);
                let var_name = sm.token_text(cmd_tok).to_string();
                let method_name = args.first().cloned();
                self.var_command_sites.push(super::state::VarCommandSite {
                    var_name,
                    method_name,
                    cmd_span: cmd_tok.span,
                    in_method,
                    argc: args.len(),
                });
            }
            TokenType::Cmd => {
                let sm = tcl_lexer::SourceMap::new(&self.source);
                let cmd_text = sm.token_text(cmd_tok).to_string();
                let method_name = args.first().cloned();
                self.cmd_command_sites.push(super::state::CmdCommandSite {
                    cmd_text,
                    method_name,
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
        // Per-item isolated proc body: `all_classes` is empty here, so the class
        // can't be resolved.  Capture the raw `(command, args)` for the two
        // instance-creation shapes and let the graft replay them against the
        // shell's full `all_classes` instead (see `pending_instances`).
        if let Some(pending) = self.pending_instances.as_mut() {
            let shape_a =
                cmd_name == "set" && args.len() >= 2 && args[1].trim_start().starts_with('[');
            let shape_b = args.len() >= 2 && args[0] == "create";
            if shape_a || shape_b {
                pending.push((cmd_name.to_owned(), args.to_vec()));
            }
            return;
        }
        // Pattern A: `set VAR [CLASS new|create ...]`.
        if cmd_name == "set"
            && args.len() >= 2
            && let Some(class_q) = self.class_from_constructor_subst(&args[1])
        {
            self.result
                .instance_classes
                .insert(args[0].clone(), class_q);
            return;
        }
        // Pattern B: `CLASS create VAR ...` — the instance
        // command is named by argv[1].
        if args.len() >= 2
            && args[0] == "create"
            && let Some(class_q) = self.resolve_user_class(cmd_name)
        {
            self.result
                .instance_classes
                .insert(args[1].clone(), class_q);
        }
    }

    /// Resolve a class reference (`Dog`, `::Dog`, or a
    /// namespace-relative form) to its qualified name when it
    /// names a user-defined class.
    fn resolve_user_class(&self, name: &str) -> Option<String> {
        if self.result.all_classes.contains_key(name) {
            return Some(name.to_string());
        }
        let qualified = format!("::{name}");
        if self.result.all_classes.contains_key(&qualified) {
            return Some(qualified);
        }
        self.result
            .all_classes
            .values()
            .find(|c| c.name == name)
            .map(|c| c.qualified_name.clone())
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
        let subcmd = words.next()?;
        if subcmd != "new" && subcmd != "create" {
            return None;
        }
        self.resolve_user_class(class)
    }
}

/// Descend a ``Cmd`` (``[…]``) substitution token and collect every
/// inner command's head as ``(name, head_span)``, recursing into nested
/// ``[...]`` substitutions and registry-resolved body arguments.
///
/// Mirrors main's ``_recurse_nested_commands``: the token is descended
/// into a child CST ([`descend_token`]) and the inner script segmented;
/// each command is then handled by [`record_command_invocations`].
/// Spans are absolute (the descent anchors the child tree at the
/// substitution's position).
fn collect_substitution_heads(
    sm: &SourceMap<'_>,
    registry: Option<&CommandRegistry>,
    cmd_tok: Token,
    config: LexerConfig,
    out: &mut Vec<(String, Span)>,
) {
    if cmd_tok.kind != TokenType::Cmd || sm.token_text(cmd_tok).is_empty() {
        return;
    }
    let descended = descend_token(sm, cmd_tok, config);
    for seg in segments_from_tree(descended.tree(), sm) {
        record_command_invocations(sm, registry, &seg, config, out);
    }
}

/// Record one (already-segmented) command's head, then recurse into its
/// nested ``[...]`` substitutions *and* its registry-resolved body
/// arguments — the combined ``_recurse_nested_commands`` +
/// ``_recurse_body_arguments`` walk main runs on every command, so a
/// command-substitution containing a control-flow command surfaces the
/// body's commands too (`[if {$c} {puts hi}]` → `if`, `puts`).
///
/// The head is recorded as main's ``argv_texts[0]`` (the `word_piece`
/// form in `texts[0]`: a ``$var`` head as ``${var}``, a ``"quoted"``
/// head unquoted, a compound ``$x$y`` head reconstructed); a ``[subst]``
/// head is left to the substitution recursion, and a ``{brace}`` head is
/// data, not a command.  Body arguments are resolved through
/// [`descend_command`] (the registry's ``arg_indices_for_role`` /
/// `iter_body_arguments`), so the body set matches the registry exactly
/// and an ``Expr`` argument is never walked as a script.
fn record_command_invocations(
    sm: &SourceMap<'_>,
    registry: Option<&CommandRegistry>,
    seg: &SegmentedCommand,
    config: LexerConfig,
    out: &mut Vec<(String, Span)>,
) {
    // Head — main records ``argv_texts[0]`` (the `word_piece` form in
    // `texts[0]`) for *every* command, whatever the head's kind: a bare
    // word, a `$var` (`${var}`), a `"quote"` (unquoted), a compound head,
    // a `[subst]` head (`[gen]` — recorded *and* descended below), or a
    // `{braced}` head (its inner text).
    if let (Some(&head), Some(name)) = (seg.argv.first(), seg.texts.first())
        && !name.is_empty()
    {
        out.push((name.clone(), head.span));
    }
    // Nested ``[...]`` substitutions in any position (args, or embedded
    // in a quoted word — both are `Cmd` tokens in the command's token
    // stream; a `{brace}` region stays opaque).
    for tok in &seg.all_tokens {
        if tok.kind == TokenType::Cmd {
            collect_substitution_heads(sm, registry, *tok, config, out);
        }
    }
    // Registry-resolved body arguments (`if` / `foreach` / `eval` / …
    // bodies).  The body's commands are inner invocations too.
    if let (Some(registry), Some(name)) = (registry, seg.texts.first()) {
        let args: Vec<&str> = seg.texts.iter().skip(1).map(String::as_str).collect();
        let arg_tokens: Vec<Token> = seg.argv.iter().skip(1).copied().collect();
        // The `switch … {pattern body …}` list-form arg is a Tcl *list*,
        // not a script — the registry still marks it `Body`, but walking
        // it as one mis-reads a pattern as a command head.  Main
        // special-cases it (`_recurse_switch_list_body`): parse the list
        // into pattern/body pairs and descend each arm *body*.
        let switch_list_idx = if name == "switch" {
            switch_list_body_index(&args)
        } else {
            None
        };
        for body in descend_command(registry, sm, name, &args, &arg_tokens, config) {
            if switch_list_idx == Some(body.index) {
                let elements = super::handlers::parse_switch_body_elements(&body.text, body.token);
                // Elements alternate pattern, body, pattern, body, … —
                // descend the (odd-indexed) arm bodies only; a `-`
                // fall-through has no body of its own.
                let mut k = 1;
                while k < elements.len() {
                    let (arm_text, arm_tok) = &elements[k];
                    if arm_text != "-" && arm_tok.kind == TokenType::Str {
                        let arm = descend_token(sm, *arm_tok, config);
                        for inner in segments_from_tree(arm.tree(), sm) {
                            record_command_invocations(sm, Some(registry), &inner, config, out);
                        }
                    }
                    k += 2;
                }
                continue;
            }
            for inner in segments_from_tree(body.descended.tree(), sm) {
                record_command_invocations(sm, Some(registry), &inner, config, out);
            }
        }
        // Expr arguments (`if` / `while` / `expr` / … conditions): a
        // command substitution inside the expression is an invocation
        // too (`if {[acl_ok]} …` → `acl_ok`).  `descend_command`
        // deliberately excludes `Expr` args (they are not scripts), so
        // handle them here — mirroring main's
        // `_recurse_expression_subcommands`.
        for index in registry.arg_indices_for_role(name, &args, ArgRole::Expr) {
            if let Some(&tok) = arg_tokens.get(index) {
                collect_expr_substitutions(sm, Some(registry), tok, config, out);
            }
        }
    }
}

/// Find the ``[…]`` command substitutions inside an expression argument
/// (`if` / `while` / `expr` conditions) and descend each — mirroring
/// main's `_recurse_expression_subcommands`.
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
    out: &mut Vec<(String, Span)>,
) {
    if expr_tok.kind != TokenType::Str || sm.token_text(expr_tok).is_empty() {
        return;
    }
    let descended = descend_token(sm, expr_tok, config);
    for seg in segments_from_tree(descended.tree(), sm) {
        for tok in &seg.all_tokens {
            if tok.kind == TokenType::Cmd {
                collect_substitution_heads(sm, registry, *tok, config, out);
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
    if cmd_tok.kind != TokenType::Cmd || sm.token_text(cmd_tok).is_empty() {
        return;
    }
    let descended = descend_token(sm, cmd_tok, config);
    for seg in segments_from_tree(descended.tree(), sm) {
        collect_segment_recursive(sm, registry, seg, config, out);
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
) {
    // Nested ``[…]`` substitutions in any word of this command.
    for tok in &seg.all_tokens {
        if tok.kind == TokenType::Cmd {
            collect_substitution_segments(sm, registry, *tok, config, out);
        }
    }
    // Registry-resolved body arguments (`[if {$c} {string index …}]`):
    // their commands are also invisible to the main walk here.
    if let Some(registry) = registry {
        let args: Vec<&str> = seg.texts.iter().skip(1).map(String::as_str).collect();
        let arg_tokens: Vec<Token> = seg.argv.iter().skip(1).copied().collect();
        for body in descend_command(registry, sm, seg.name(), &args, &arg_tokens, config) {
            for inner in segments_from_tree(body.descended.tree(), sm) {
                collect_segment_recursive(sm, Some(registry), inner, config, out);
            }
        }
    }
    out.push(seg);
}

/// Mirror of main's `_switch_list_body_index`: for the
/// ``switch ?options? string {pattern body …}`` form, the index (into
/// `args`, 0-based, excluding the command name) of the single braced
/// pattern/body list.  `None` for the separate-args form
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
/// `log "got { [HTTP::uri] }"` still executes — and must still be scanned
/// (Codex review, PR #639).
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
/// Mirrors the ``_iter_nested_invocations`` walk in
/// ``_AnalyserCommandsMixin._record_command_invocation``.
/// Returns ``(name, byte_offset_in_text)`` pairs.  The caller
/// adds the enclosing token's source-span start to obtain an
/// absolute offset.
pub(crate) fn scan_nested_command_heads(text: &str) -> Vec<(String, u32)> {
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
                    for (name, off_in_inner) in scan_nested_command_heads(inner) {
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
        // recorded. (Unknown-command diagnostic emission is C41d4.)
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
}
