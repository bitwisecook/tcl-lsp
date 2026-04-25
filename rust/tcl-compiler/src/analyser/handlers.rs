//! Per-command handlers (variable-write subset) — Rust port of
//! the variable-mutation handlers in
//! ``core/analysis/_analyser/_handlers.py``.
//!
//! C41b1 lands the variable-write trio:
//!
//! - [`Analyser::handle_set_command`] — `set var ?value?`
//! - [`Analyser::handle_var_declaration_command`] —
//!   `variable name ?value? ...?` and `global name...`
//! - [`Analyser::handle_incr_command`] — `incr var ?amount?`
//!
//! Subsequent C41b strips fill in the rest of `_handlers.py`:
//!
//! - **C41b2** — `_handle_proc_command` (proc-body walker).
//! - **C41b3** — `_handle_namespace_eval_command` and
//!   `_handle_namespace_ensemble`.
//! - **C41b4** — `_handle_foreach_command`,
//!   `_handle_for_command`, `_handle_switch_command`.
//! - **C41b5** — `_handle_catch_command`,
//!   `_handle_try_command`.
//! - **C41b6** — `_handle_interp_alias`,
//!   `_handle_oo_objdefine`, `_resolve_alias`.

use tcl_lexer::{Token, TokenType};

use super::state::Analyser;
use super::types::ProcDef;
use super::utils::parse_param_list;

/// Build a fully-qualified Tcl proc / class name from a namespace
/// prefix and a possibly-relative name.
///
/// Mirrors `qualify` in `signature_scan/handlers.rs:33-41` (which
/// itself ports `_qualify` from
/// `core/analysis/signature_scan.py`). `ns_prefix` is the
/// namespace **without** a leading `::` — the convention used
/// throughout the analyser walker.
pub(super) fn qualify(ns_prefix: &str, name: &str) -> String {
    if name.starts_with("::") {
        name.to_string()
    } else if ns_prefix.is_empty() {
        format!("::{name}")
    } else {
        format!("::{ns_prefix}::{name}")
    }
}

impl Analyser {
    /// Handle the `set` command: `set var ?value?`.
    ///
    /// Mirrors `_handle_set_command` in
    /// `core/analysis/_analyser/_handlers.py:51-70`. Defines the
    /// target variable in the scope at `scope_path` and tracks
    /// the value as a constant string when the value is a
    /// single-token literal (no interpolation, no command sub).
    ///
    /// `single_token_word` parallels `args` and `arg_tokens` —
    /// `true` when the corresponding word is a single atomic
    /// token. It's the Rust replacement for Python's
    /// `value_token.text == args[1]` check, which boils down to
    /// "is this word's text the same as a single token's raw
    /// text?".
    ///
    /// **C41c hook.** The Python source also calls
    /// `_AnalyserProcMixin._handle_set` (which lives in
    /// `_proc.py:177-191`) for the proc-scope variant. That
    /// inner walk is ported in **C41c2**; for now this handler
    /// only does the const-string tracking + the
    /// outer-scope define.
    pub fn handle_set_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        single_token_word: &[bool],
        scope_path: &[usize],
    ) {
        if cmd_name != "set" || args.is_empty() {
            return;
        }

        // Define the variable being assigned. The Python source
        // does this through `_handle_set` (deferred to C41c2);
        // we inline the basic case here so handlers built on top
        // see the var in scope without requiring C41c2 to land
        // first.
        if let Some(name_tok) = arg_tokens.first() {
            self.define_var(&args[0], *name_tok, scope_path, true, None);
        }

        // Track constant-string assignments for regex propagation.
        if args.len() < 2 || arg_tokens.len() < 2 {
            return;
        }
        let value_token = arg_tokens[1];
        let value_is_single_token = single_token_word.get(1).copied().unwrap_or(false);
        let value_token_kind = value_token.kind;
        if value_is_single_token && matches!(value_token_kind, TokenType::Esc | TokenType::Str) {
            self.set_const_string(&args[0], args[1].clone(), value_token.span, scope_path);
        } else {
            self.clear_const_string(&args[0], scope_path);
        }
    }

    /// Handle `variable` / `global` declarations.
    ///
    /// Mirrors `_handle_var_declaration_command` in
    /// `core/analysis/_analyser/_handlers.py:72-95`.
    ///
    /// - `global` takes a flat list of names; each gets a var
    ///   binding with `warn_if_unused = false` (declared, not
    ///   "set but unused").
    /// - `variable` takes alternating `name ?value?` pairs; only
    ///   the names get bindings. The optional value words are
    ///   skipped (the IR pass handles their assignment if the
    ///   value form actually fires).
    pub fn handle_var_declaration_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if !matches!(cmd_name, "variable" | "global") || args.is_empty() {
            return;
        }

        if cmd_name == "global" {
            for (i, name) in args.iter().enumerate() {
                if let Some(tok) = arg_tokens.get(i) {
                    self.define_var(name, *tok, scope_path, false, None);
                }
            }
            return;
        }

        // `variable name ?value? name ?value? ...`
        let mut i = 0;
        while i < args.len() {
            if let Some(tok) = arg_tokens.get(i) {
                self.define_var(&args[i], *tok, scope_path, false, None);
            }
            i += if i + 1 < args.len() { 2 } else { 1 };
        }
    }

    /// Handle the `proc` command: `proc NAME PARAMS BODY`.
    ///
    /// Mirrors `_handle_proc_command` in
    /// `core/analysis/_analyser/_handlers.py:39-49`. Returns
    /// `true` when the command was actually handled (callers use
    /// the bool to decide whether further processing is needed),
    /// `false` when the input doesn't match the expected shape.
    ///
    /// **C41b2 baseline + C41c1 hook.** Python delegates to
    /// `_AnalyserProcMixin._handle_proc` (which lives in
    /// `_proc.py:46-176` and does the proc-body walk + parameter-
    /// trait inference). The proc-body walk lands in **C41c1**;
    /// for now this strip records the basic [`ProcDef`] shape
    /// (qualified name, params, name + body spans, doc-comment)
    /// in both `current_scope.procs` and `result.all_procs` so
    /// downstream consumers (`signature_scan` parity, the LSP
    /// rename feature, the workspace index) see a correct proc
    /// list immediately. The deeper body walk + scope creation
    /// is deferred.
    pub fn handle_proc_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if cmd_name != "proc" || args.len() < 3 || arg_tokens.len() < 3 {
            return false;
        }

        let raw_name = &args[0];
        let ns_prefix = self.namespace_from_scope_path(scope_path);
        // Strip the leading `::` for the qualify helper, which
        // expects an unprefixed namespace.
        let ns_for_qualify = ns_prefix.trim_start_matches(':');
        let qualified = qualify(ns_for_qualify, raw_name);
        let simple = qualified.rsplit("::").next().unwrap_or("").to_string();
        let name_span = arg_tokens[0].span;
        let body_span = arg_tokens[2].span;

        let params = parse_param_list(&args[1]);
        let doc = std::mem::take(&mut self.last_comment);

        let proc = ProcDef {
            name: simple,
            qualified_name: qualified.clone(),
            params,
            name_span,
            body_span,
            doc,
        };

        // Register globally and in the current scope.
        self.result
            .all_procs
            .insert(qualified.clone(), proc.clone());
        let path = scope_path.to_vec();
        if let Some(scope) = super::scope::scope_at_mut(&mut self.result.global_scope, &path) {
            scope.procs.insert(qualified, proc);
        }

        // C41c1 hook: the proc-body walk happens here. For now
        // the body span is recorded so consumers can post-process.
        true
    }

    /// Handle `namespace eval`: opens a new namespace scope and
    /// schedules its body for analysis.
    ///
    /// Mirrors `_handle_namespace_eval_command` in
    /// `core/analysis/_analyser/_handlers.py:97-118`. Returns
    /// `true` when the command was handled.
    ///
    /// **C41f1 hook.** Python recurses into the body via
    /// `_analyse_body`, which lives in `_core.py` (the orchestrator
    /// layer). The Rust port creates the child scope and stores
    /// the body span; the deeper body recursion is wired in
    /// **C41f1** when the analyser orchestration lands. For now
    /// the namespace scope is added so downstream handlers can
    /// see qualified names resolve through it.
    pub fn handle_namespace_eval_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if cmd_name != "namespace" || args.len() < 2 || args[0] != "eval" {
            return false;
        }
        let ns_name = args[1].clone();
        let body_span = arg_tokens.get(2).map(|t| t.span);

        let mut child = super::types::Scope::new(super::types::ScopeKind::Namespace, ns_name);
        child.body_span = body_span;

        let path = scope_path.to_vec();
        let Some(parent) = super::scope::scope_at_mut(&mut self.result.global_scope, &path) else {
            return false;
        };
        parent.children.push(child);
        true
    }

    /// Handle `namespace ensemble create` — record the namespace as
    /// an ensemble so its tail names become valid commands.
    ///
    /// Mirrors `_handle_namespace_ensemble` in
    /// `core/analysis/_analyser/_handlers.py:254-268`.
    pub fn handle_namespace_ensemble(
        &mut self,
        cmd_name: &str,
        args: &[String],
        scope_path: &[usize],
    ) {
        if cmd_name != "namespace" || args.len() < 2 {
            return;
        }
        if args[0] != "ensemble" || args[1] != "create" {
            return;
        }
        let ns = self.namespace_from_scope_path(scope_path);
        if !ns.is_empty() && ns != "::" {
            self.ensemble_namespaces.insert(ns);
        }
    }

    /// Handle the `incr` command: `incr var ?amount?`.
    ///
    /// Mirrors `_handle_incr_command` in
    /// `core/analysis/_analyser/_handlers.py:215-223`. `incr` is
    /// safe-on-uninit (it initialises the variable to 0 if not
    /// yet set), so the var binding is created with
    /// `warn_if_unused = true` — the diagnostic emitter will
    /// still flag a `set`-only-no-read variable, but won't flag
    /// an `incr`-only-no-read one.
    pub fn handle_incr_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) {
        if cmd_name != "incr" {
            return;
        }
        if let (Some(name), Some(tok)) = (args.first(), arg_tokens.first()) {
            self.define_var(name, *tok, scope_path, true, None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::Span;

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

    // -- handle_set_command ------------------------------------------

    #[test]
    fn handle_set_defines_variable() {
        let mut a = Analyser::new();
        a.handle_set_command(
            "set",
            &["x".to_string(), "1".to_string()],
            &[esc_tok(span(0, 1)), esc_tok(span(2, 3))],
            &[true, true],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("x"));
    }

    #[test]
    fn handle_set_tracks_single_token_literal_value() {
        let mut a = Analyser::new();
        a.handle_set_command(
            "set",
            &["x".to_string(), "hello".to_string()],
            &[esc_tok(span(0, 1)), esc_tok(span(2, 7))],
            &[true, true],
            &[],
        );
        assert_eq!(a.lookup_const_string("x", &[]), Some("hello"));
    }

    #[test]
    fn handle_set_tracks_braced_string_value() {
        let mut a = Analyser::new();
        a.handle_set_command(
            "set",
            &["x".to_string(), "hello world".to_string()],
            &[esc_tok(span(0, 1)), str_tok(span(2, 15))],
            &[true, true],
            &[],
        );
        assert_eq!(a.lookup_const_string("x", &[]), Some("hello world"));
    }

    #[test]
    fn handle_set_clears_const_string_for_interpolated_value() {
        let mut a = Analyser::new();
        // Pre-seed a constant tracking entry.
        a.set_const_string("x", "old".to_string(), span(0, 0), &[]);
        // Re-assign with a multi-token (interpolation) value —
        // single_token_word[1] is false, so const_string is cleared.
        a.handle_set_command(
            "set",
            &["x".to_string(), "$other".to_string()],
            &[esc_tok(span(0, 1)), esc_tok(span(2, 8))],
            &[true, false],
            &[],
        );
        assert_eq!(a.lookup_const_string("x", &[]), None);
    }

    #[test]
    fn handle_set_no_value_only_defines_var() {
        let mut a = Analyser::new();
        a.handle_set_command(
            "set",
            &["x".to_string()],
            &[esc_tok(span(0, 1))],
            &[true],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("x"));
        assert_eq!(a.lookup_const_string("x", &[]), None);
    }

    #[test]
    fn handle_set_wrong_command_no_op() {
        let mut a = Analyser::new();
        a.handle_set_command(
            "puts",
            &["x".to_string(), "1".to_string()],
            &[esc_tok(span(0, 1)), esc_tok(span(2, 3))],
            &[true, true],
            &[],
        );
        assert!(a.result.global_scope.variables.is_empty());
    }

    // -- handle_var_declaration_command -----------------------------

    #[test]
    fn handle_global_defines_each_name() {
        let mut a = Analyser::new();
        a.handle_var_declaration_command(
            "global",
            &["x".to_string(), "y".to_string(), "z".to_string()],
            &[
                esc_tok(span(0, 1)),
                esc_tok(span(2, 3)),
                esc_tok(span(4, 5)),
            ],
            &[],
        );
        for name in ["x", "y", "z"] {
            assert!(a.result.global_scope.variables.contains_key(name));
            assert!(!a.result.global_scope.variables[name].warn_if_unused);
        }
    }

    #[test]
    fn handle_variable_defines_only_names_skipping_values() {
        let mut a = Analyser::new();
        // `variable x 1 y 2 z` — names at 0, 2, 4; values at 1, 3.
        a.handle_var_declaration_command(
            "variable",
            &[
                "x".to_string(),
                "1".to_string(),
                "y".to_string(),
                "2".to_string(),
                "z".to_string(),
            ],
            &[
                esc_tok(span(0, 1)),
                esc_tok(span(2, 3)),
                esc_tok(span(4, 5)),
                esc_tok(span(6, 7)),
                esc_tok(span(8, 9)),
            ],
            &[],
        );
        for name in ["x", "y", "z"] {
            assert!(a.result.global_scope.variables.contains_key(name));
        }
        // Numbers should NOT be variable names.
        assert!(!a.result.global_scope.variables.contains_key("1"));
        assert!(!a.result.global_scope.variables.contains_key("2"));
    }

    #[test]
    fn handle_variable_single_name_no_value() {
        let mut a = Analyser::new();
        a.handle_var_declaration_command(
            "variable",
            &["x".to_string()],
            &[esc_tok(span(0, 1))],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("x"));
    }

    #[test]
    fn handle_var_declaration_wrong_command_no_op() {
        let mut a = Analyser::new();
        a.handle_var_declaration_command("set", &["x".to_string()], &[esc_tok(span(0, 1))], &[]);
        assert!(a.result.global_scope.variables.is_empty());
    }

    // -- handle_proc_command ----------------------------------------

    #[test]
    fn handle_proc_records_proc_at_global() {
        let mut a = Analyser::new();
        let handled = a.handle_proc_command(
            "proc",
            &["foo".to_string(), "a b".to_string(), "set x $a".to_string()],
            &[
                esc_tok(span(5, 8)),
                esc_tok(span(9, 14)),
                str_tok(span(15, 25)),
            ],
            &[],
        );
        assert!(handled);
        assert!(a.result.all_procs.contains_key("::foo"));
        let proc = &a.result.all_procs["::foo"];
        assert_eq!(proc.name, "foo");
        assert_eq!(proc.qualified_name, "::foo");
        assert_eq!(proc.params.len(), 2);
        assert_eq!(proc.params[0].name, "a");
        assert_eq!(proc.params[1].name, "b");
    }

    #[test]
    fn handle_proc_qualifies_under_namespace() {
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "ns1"));
        let handled = a.handle_proc_command(
            "proc",
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[0],
        );
        assert!(handled);
        assert!(a.result.all_procs.contains_key("::ns1::foo"));
    }

    #[test]
    fn handle_proc_absolute_name_rebases() {
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "outer"));
        let handled = a.handle_proc_command(
            "proc",
            &["::other::foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 17)),
                str_tok(span(18, 20)),
                str_tok(span(21, 23)),
            ],
            &[0],
        );
        assert!(handled);
        // Absolute name rebases — does NOT nest under outer.
        assert!(a.result.all_procs.contains_key("::other::foo"));
        assert!(!a.result.all_procs.contains_key("::outer::other::foo"));
    }

    #[test]
    fn handle_proc_consumes_last_comment_as_doc() {
        let mut a = Analyser::new();
        a.last_comment = "doc string".to_string();
        a.handle_proc_command(
            "proc",
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(0, 3)),
                str_tok(span(4, 6)),
                str_tok(span(7, 9)),
            ],
            &[],
        );
        assert_eq!(a.result.all_procs["::foo"].doc, "doc string");
        // last_comment is consumed.
        assert!(a.last_comment.is_empty());
    }

    #[test]
    fn handle_proc_too_few_args_returns_false() {
        let mut a = Analyser::new();
        let handled =
            a.handle_proc_command("proc", &["foo".to_string()], &[esc_tok(span(0, 3))], &[]);
        assert!(!handled);
        assert!(a.result.all_procs.is_empty());
    }

    #[test]
    fn handle_proc_wrong_command_returns_false() {
        let mut a = Analyser::new();
        let handled = a.handle_proc_command(
            "puts",
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(0, 3)),
                str_tok(span(4, 6)),
                str_tok(span(7, 9)),
            ],
            &[],
        );
        assert!(!handled);
        assert!(a.result.all_procs.is_empty());
    }

    // -- handle_namespace_eval_command ------------------------------

    #[test]
    fn handle_namespace_eval_creates_child_scope() {
        let mut a = Analyser::new();
        let handled = a.handle_namespace_eval_command(
            "namespace",
            &[
                "eval".to_string(),
                "ns1".to_string(),
                "proc inner {} {}".to_string(),
            ],
            &[
                esc_tok(span(10, 14)),
                esc_tok(span(15, 18)),
                str_tok(span(19, 35)),
            ],
            &[],
        );
        assert!(handled);
        assert_eq!(a.result.global_scope.children.len(), 1);
        assert_eq!(a.result.global_scope.children[0].name, "ns1");
        assert_eq!(
            a.result.global_scope.children[0].kind,
            crate::analyser::types::ScopeKind::Namespace,
        );
    }

    #[test]
    fn handle_namespace_eval_records_body_span() {
        let mut a = Analyser::new();
        a.handle_namespace_eval_command(
            "namespace",
            &["eval".to_string(), "ns1".to_string(), String::new()],
            &[
                esc_tok(span(10, 14)),
                esc_tok(span(15, 18)),
                str_tok(span(19, 35)),
            ],
            &[],
        );
        assert_eq!(
            a.result.global_scope.children[0].body_span,
            Some(span(19, 35))
        );
    }

    #[test]
    fn handle_namespace_eval_wrong_subcommand_returns_false() {
        let mut a = Analyser::new();
        let handled = a.handle_namespace_eval_command(
            "namespace",
            &["import".to_string(), "::tcl::*".to_string()],
            &[esc_tok(span(0, 6)), esc_tok(span(7, 16))],
            &[],
        );
        assert!(!handled);
        assert!(a.result.global_scope.children.is_empty());
    }

    // -- handle_namespace_ensemble ----------------------------------

    #[test]
    fn handle_namespace_ensemble_records_in_set() {
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "myns"));
        a.handle_namespace_ensemble(
            "namespace",
            &["ensemble".to_string(), "create".to_string()],
            &[0],
        );
        assert!(a.ensemble_namespaces.contains("::myns"));
    }

    #[test]
    fn handle_namespace_ensemble_global_scope_no_op() {
        let mut a = Analyser::new();
        a.handle_namespace_ensemble(
            "namespace",
            &["ensemble".to_string(), "create".to_string()],
            &[],
        );
        assert!(a.ensemble_namespaces.is_empty());
    }

    #[test]
    fn handle_namespace_ensemble_wrong_subcommand_no_op() {
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "myns"));
        a.handle_namespace_ensemble("namespace", &["eval".to_string(), "myns".to_string()], &[0]);
        assert!(a.ensemble_namespaces.is_empty());
    }

    // -- handle_incr_command ----------------------------------------

    #[test]
    fn handle_incr_defines_var() {
        let mut a = Analyser::new();
        a.handle_incr_command(
            "incr",
            &["counter".to_string()],
            &[esc_tok(span(0, 7))],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("counter"));
        // incr-defined vars warn_if_unused = true (so a `set`-only
        // var pattern still fires; an `incr`-only-no-read does too).
        assert!(a.result.global_scope.variables["counter"].warn_if_unused);
    }

    #[test]
    fn handle_incr_with_amount() {
        let mut a = Analyser::new();
        a.handle_incr_command(
            "incr",
            &["counter".to_string(), "5".to_string()],
            &[esc_tok(span(0, 7)), esc_tok(span(8, 9))],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("counter"));
    }

    #[test]
    fn handle_incr_no_args_no_op() {
        let mut a = Analyser::new();
        a.handle_incr_command("incr", &[], &[], &[]);
        assert!(a.result.global_scope.variables.is_empty());
    }

    #[test]
    fn handle_incr_wrong_command_no_op() {
        let mut a = Analyser::new();
        a.handle_incr_command("set", &["counter".to_string()], &[esc_tok(span(0, 7))], &[]);
        assert!(a.result.global_scope.variables.is_empty());
    }
}
