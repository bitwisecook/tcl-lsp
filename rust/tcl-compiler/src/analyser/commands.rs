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

use tcl_lexer::Token;

use super::state::Analyser;

impl Analyser {
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

        // Handler-by-handler dispatch. Each returning-bool
        // handler is consulted in turn; first match wins. The
        // void-returning handlers run unconditionally (their
        // own internal cmd-name guard rejects mismatches).

        // proc — registers the proc record + scope.
        if self.handle_proc_command(cmd_name, args, arg_tokens, scope_path) {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{Span, TokenType};

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
