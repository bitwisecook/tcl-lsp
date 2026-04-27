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

use tcl_lexer::{Token, TokenType};

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
    /// Body recursion does **not** use Seg2 recovery — recovery
    /// only fires at the top level (matches Python's
    /// `_analyse_body` vs. `_analyse_body_inner` split).
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
        let body_commands = crate::segmenter::segment_commands_with_offset(body_text, base_offset);
        let total = body_commands.len();
        let mut cmd_idx: usize = 0;
        while cmd_idx < total {
            let cmd_ref = &body_commands[cmd_idx];
            if cmd_ref.argv.is_empty() {
                cmd_idx += 1;
                continue;
            }
            if cmd_ref.is_partial {
                // **C41e5.** Stolen-close-brace detection ⇒ E103;
                // otherwise the generic E200 fires so the user
                // still sees a parse-error diagnostic.
                if !self.detect_stolen_close_brace(cmd_ref) {
                    self.emit_partial_command_diagnostic(cmd_ref);
                }
                cmd_idx += 1;
                continue;
            }
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
            self.process_command(&cmd.texts, &cmd.argv, &cmd.single_token_word, scope_path);
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
    /// W125 / IRULE5005 emission, arity checks, sub-command
    /// resolution, command-invocation recording with
    /// resolved-qname annotation.
    pub fn process_command(
        &mut self,
        argv_texts: &[String],
        arg_tokens_in: &[Token],
        single_token_word: &[bool],
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
        self.result.command_invocations.push(
            crate::signature_scan::types::SignatureCommandInvocation {
                name: cmd_name.to_string(),
                range: cmd_tok.span,
            },
        );

        // iRules ``call PROC ARG...`` — record an additional
        // ``CommandInvocation`` for the target proc so that
        // references, rename, and call-hierarchy see through the
        // indirection.  Mirrors
        // ``_AnalyserCommandsMixin._process_command`` line 231 in
        // ``core/analysis/_analyser/_commands.py``.
        if cmd_name == "call" && self.dialect == "f5-irules" {
            if let (Some(target_name), Some(target_tok)) =
                (args.first(), arg_tokens_in.get(1).copied())
            {
                self.result.command_invocations.push(
                    crate::signature_scan::types::SignatureCommandInvocation {
                        name: target_name.clone(),
                        range: target_tok.span,
                    },
                );
            }
        }

        // Walk each argument's source text for ``[cmd ...]``
        // substitutions and record each nested command's head as
        // its own ``CommandInvocation``.  Mirrors the
        // ``_iter_nested_invocations`` walk in
        // ``_AnalyserCommandsMixin._record_command_invocation``
        // (Python).  Without this, calls embedded inside argument
        // expressions (``set x [helper $foo]``,
        // ``puts "got [count $items]"`` …) aren't tracked, which
        // breaks workspace usage counts, find-references, rename,
        // and call-hierarchy.
        for arg_tok in arg_tokens_in.iter().skip(1) {
            let arg_start = arg_tok.span.start();
            let arg_end = arg_tok.span.end() as usize;
            let src_bytes = self.source.as_bytes();
            if arg_start as usize >= src_bytes.len() || arg_end > src_bytes.len() {
                continue;
            }
            let arg_src = &self.source[arg_start as usize..arg_end];
            if matches!(arg_tok.kind, TokenType::Cmd) {
                // ``Cmd`` tokens cover the inner text (no
                // surrounding ``[`` / ``]``) when ``content_offset
                // = 1`` — but the segmenter's ``Cmd`` span starts
                // at the ``[`` and ends one past the inner text,
                // *without* the closing ``]``.  Strip the leading
                // ``[`` (skip ``content_offset`` bytes) and pass
                // the inner content directly to
                // ``first_command_head``; then recurse into the
                // inner text for nested substitutions.
                let inner_off = arg_tok.content_offset as usize;
                if inner_off <= arg_src.len() {
                    let inner = &arg_src[inner_off..];
                    if let Some((head, head_off)) = first_command_head(inner) {
                        let abs_start = arg_start + (inner_off + head_off) as u32;
                        let abs_end = abs_start + head.len() as u32;
                        self.result.command_invocations.push(
                            crate::signature_scan::types::SignatureCommandInvocation {
                                name: head.to_string(),
                                range: tcl_lexer::Span::new(abs_start, abs_end),
                            },
                        );
                    }
                    for (name, off) in scan_nested_command_heads(inner) {
                        let abs_start = arg_start + (inner_off + off as usize) as u32;
                        let abs_end = abs_start + name.len() as u32;
                        self.result.command_invocations.push(
                            crate::signature_scan::types::SignatureCommandInvocation {
                                name,
                                range: tcl_lexer::Span::new(abs_start, abs_end),
                            },
                        );
                    }
                }
            } else {
                // For ``Esc`` (bareword / quoted) and ``Str``
                // (braced) tokens the span covers the full token
                // including any quotes / braces; ``scan_nested_*``
                // walks ``[…]`` substitutions inside.
                for (name, off) in scan_nested_command_heads(arg_src) {
                    let abs_start = arg_start + off;
                    let abs_end = abs_start + name.len() as u32;
                    self.result.command_invocations.push(
                        crate::signature_scan::types::SignatureCommandInvocation {
                            name,
                            range: tcl_lexer::Span::new(abs_start, abs_end),
                        },
                    );
                }
            }
        }

        // **C41d3.** Record variable-as-command and command-sub-as-
        // command sites so the post-walk W307 / W308 emitters can
        // resolve them.  Mirrors the inline recording in
        // ``_AnalyserCommandsMixin._process_command``
        // (``core/analysis/_analyser/_commands.py:182-198``).
        // The token-text is resolved via ``SourceMap::token_text`` —
        // the same helper that strips the ``$`` / ``${...}`` prefix
        // for VAR tokens.
        let in_method = false; // OO method-context detection lands in C41e.
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

        // Side-effect-only handlers. Same idempotent pattern.
        self.handle_namespace_ensemble(cmd_name, args, scope_path);
        self.handle_interp_alias(cmd_name, args);
        self.handle_oo_objdefine(cmd_name, args);
        self.handle_package_command(cmd_name, cmd_tok, args, arg_tokens);
        self.handle_source_command(cmd_name, args, arg_tokens);
        self.handle_namespace_import_command(cmd_name, args, arg_tokens, scope_path);
        self.handle_tcllib_import_wrapper(cmd_name, cmd_tok, args, scope_path);
        self.handle_auto_path_command(cmd_name, args, arg_tokens);
        self.handle_regex_pattern_capture(cmd_name, args, arg_tokens);

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
        if let Some(registry) = self.registry.as_ref() {
            let body_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            let body_indices = registry.arg_indices_for_role(
                cmd_name,
                &body_args,
                tcl_registry::arg_role::ArgRole::Body,
            );
            if !body_indices.is_empty() {
                let prev_event = self.current_event.clone();
                if cmd_name == "when" && !args.is_empty() {
                    self.current_event = Some(args[0].clone());
                }
                let is_conditional = matches!(cmd_name, "if" | "try");
                if is_conditional {
                    self.conditional_depth += 1;
                }
                for idx in body_indices {
                    if let (Some(body_text), Some(body_tok)) =
                        (args.get(idx), arg_tokens.get(idx).copied())
                    {
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
        }
    }
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
                    if let Some((name, head_offset_in_inner)) = first_command_head(inner) {
                        let abs_offset = inner_start + head_offset_in_inner;
                        out.push((name.to_string(), abs_offset as u32));
                    }
                    // Recurse into the inner text — nested ``[...]``
                    // substitutions inside this one also produce
                    // invocations.
                    for (name, off_in_inner) in scan_nested_command_heads(inner) {
                        out.push((name, inner_start as u32 + off_in_inner));
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
        );
        // No handler matched; no procs, vars, classes, or aliases
        // recorded. (Unknown-command diagnostic emission is C41d4.)
        assert!(a.result.all_procs.is_empty());
        assert!(a.result.global_scope.variables.is_empty());
    }

    #[test]
    fn process_empty_argv_is_no_op() {
        let mut a = Analyser::new();
        a.process_command(&[], &[], &[], &[]);
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
        );
        assert!(a.command_aliases.contains_key("::myset"));
    }
}
