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

use crate::alias::{detect_interp_alias, resolve_alias};
use crate::signature_scan::types::SignatureCommandAlias;

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
    /// `core/analysis/_analyser/_handlers.py:39-49` plus the body
    /// walk in `_AnalyserProcMixin._handle_proc`
    /// (`_proc.py:46-176`). Returns `true` when the command was
    /// handled (callers use the bool to decide whether further
    /// processing is needed), `false` when the input doesn't
    /// match the expected shape.
    ///
    /// **C41b2 baseline + C41c1.** Records the [`ProcDef`] in
    /// both `scope.procs` (keyed by simple name) and
    /// `result.all_procs` (keyed by qualified name).  When the
    /// body is a braced literal, opens a fresh
    /// [`ScopeKind::Proc`] child scope, defines each parameter
    /// in it, and re-segments the body via
    /// [`crate::segmenter::segment_commands_with_offset`] —
    /// every body command is dispatched through
    /// [`Analyser::process_command`] with the new scope path.
    /// Body recursion does **not** invoke segmenter recovery —
    /// that fires only at the top level (mirrors Python's
    /// `_analyse_body` vs. `_analyse_body_inner` split).
    /// Dynamic bodies (`$body`, `[gen]`) skip the walk because
    /// they cannot be statically re-segmented; the proc record
    /// is still emitted so downstream consumers see the
    /// signature.
    ///
    /// W113 (proc shadows built-in), parameter-trait inference,
    /// and the user-defined ``unknown`` proc detection from
    /// `_proc.py` are deferred to **C41d** / future strips —
    /// this strip is structural only.
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
        let name_tok = arg_tokens[0];
        let name_span = name_tok.span;
        let body_tok = arg_tokens[2];
        let body_span = body_tok.span;

        let params = parse_param_list(&args[1]);
        let doc = std::mem::take(&mut self.last_comment);

        let proc = ProcDef {
            name: simple,
            qualified_name: qualified.clone(),
            params: params.clone(),
            name_span,
            body_span,
            doc,
        };

        // Register globally and in the current scope. Mirrors
        // ``_proc.py:111-112`` — ``scope.procs`` is keyed by the
        // *simple* (unqualified) proc name (so per-scope lookup
        // and shadowing rules work locally), while
        // ``result.all_procs`` is keyed by the fully-qualified
        // name. The full qualified name is still on
        // ``ProcDef.qualified_name`` for callers that need it.
        self.result
            .all_procs
            .insert(qualified.clone(), proc.clone());
        let simple_key = proc.name.clone();
        let path = scope_path.to_vec();
        if let Some(scope) = super::scope::scope_at_mut(&mut self.result.global_scope, &path) {
            scope.procs.insert(simple_key, proc);
        }

        // **C41c1.** Walk the body in a fresh proc scope when the
        // body is a braced literal. ``raw_name`` is used as the
        // proc-scope name to mirror Python's
        // ``Scope(kind="proc", name=proc_name, ...)``
        // (``_proc.py:115``); ``define_var`` keys
        // ``result.all_variables`` on ``"<scope_name>::<var>"``,
        // so matching the Python scope name is what keeps that
        // map in parity.
        if body_tok.kind == TokenType::Str {
            let proc_scope_idx = {
                let parent = super::scope::scope_at_mut(&mut self.result.global_scope, &path)
                    .expect("scope_path resolved when registering proc must still resolve");
                let mut child =
                    super::types::Scope::new(super::types::ScopeKind::Proc, raw_name.clone());
                child.body_span = Some(body_span);
                parent.children.push(child);
                parent.children.len() - 1
            };
            let mut child_path = path.clone();
            child_path.push(proc_scope_idx);

            // Parameters become locals in the proc scope. Python
            // anchors each param's definition range to the proc
            // *name* token (`_proc.py:120-124`) — there's no
            // per-parameter span available without re-tokenising
            // the param-list literal. Mirror the same coarse
            // anchor here; per-param spans can land in a follow-up.
            for p in &params {
                self.define_var(&p.name, name_tok, &child_path, false, None);
            }

            // Save / restore last_comment around the body walk so
            // a doc-comment inside the proc body doesn't bleed to
            // whatever follows the proc at the outer scope. Mirrors
            // ``saved_comment = self._last_comment`` in
            // ``_proc.py:128-131``.
            let saved_comment = std::mem::take(&mut self.last_comment);

            // Body recursion: re-segment using
            // ``segment_commands_with_offset`` (no recovery — recovery
            // is top-level only, matching Python's
            // ``_analyse_body_inner`` semantics). The body's tokens
            // get rebased from local-offset space into the outer
            // source's offset space via ``base_offset``.
            self.body_depth += 1;
            let body_text = args[2].clone();
            let base_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
            let body_commands =
                crate::segmenter::segment_commands_with_offset(&body_text, base_offset);
            for cmd in body_commands {
                if cmd.is_partial || cmd.argv.is_empty() {
                    continue;
                }
                self.process_command(&cmd.texts, &cmd.argv, &cmd.single_token_word, &child_path);
            }
            self.body_depth -= 1;

            self.last_comment = saved_comment;
        }

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

    /// Define a list of variables from a varList token (e.g. the
    /// loop-variable list of `foreach`). Mirrors
    /// `_define_vars_from_list` in
    /// `core/analysis/_analyser/_scope.py:81-124`.
    ///
    /// **Simplified port.** Python uses
    /// `position_from_relative` to compute a per-name range
    /// inside the varList token's text. Rust uses the parent
    /// token's span for every defined var — a coarser
    /// approximation that's acceptable at this strip; per-name
    /// span resolution lands when ``position_from_relative``
    /// gets a Rust port (deferred to a follow-up).
    fn define_vars_from_list(&mut self, var_list_text: &str, tok: Token, scope_path: &[usize]) {
        for name in var_list_text.split_whitespace() {
            self.define_var(name, tok, scope_path, true, None);
        }
    }

    /// Handle `foreach var list body` (and the `foreach_in_collection`
    /// dialect variant).
    ///
    /// Mirrors `_handle_foreach_command` in
    /// `core/analysis/_analyser/_handlers.py:120-142`. Defines the
    /// loop-variable list in the active scope; the body recursion
    /// is deferred to **C41f1**.
    pub fn handle_foreach_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if !matches!(cmd_name, "foreach" | "foreach_in_collection") {
            return false;
        }
        if args.len() < 3 {
            return false;
        }
        if let Some(tok) = arg_tokens.first() {
            self.define_vars_from_list(&args[0], *tok, scope_path);
        }
        true
    }

    /// Handle `for init test next body`.
    ///
    /// Mirrors `_handle_for_command` in
    /// `core/analysis/_analyser/_handlers.py:144-162`. Body
    /// recursion deferred to **C41f1**.
    pub fn handle_for_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        _arg_tokens: &[Token],
        _scope_path: &[usize],
    ) -> bool {
        if cmd_name != "for" || args.len() < 4 {
            return false;
        }
        // Body / test / next recursion lands in C41f1.
        true
    }

    /// Handle `switch ?options? string ?pattern body? ...`.
    ///
    /// Mirrors `_handle_switch_command` in
    /// `core/analysis/_analyser/_handlers.py:164-177`. Arity
    /// checking now lives in `compiler_checks::arity_checks` via
    /// the IR; this handler delegates the body walk to the
    /// `_handle_switch` proc-scope variant in
    /// ``_proc.py:192-258``, deferred to **C41c2**.
    pub fn handle_switch_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        _arg_tokens: &[Token],
        _scope_path: &[usize],
    ) -> bool {
        if cmd_name != "switch" || args.len() < 2 {
            return false;
        }
        // C41c2: delegate to handle_switch for arm body recursion.
        true
    }

    /// Handle `catch SCRIPT ?RESULTVAR? ?OPTIONSVAR?`.
    ///
    /// Mirrors `_handle_catch_command` in
    /// `core/analysis/_analyser/_handlers.py:179-198`. Defines
    /// the optional `RESULTVAR` and `OPTIONSVAR` bindings (they
    /// receive values when the body throws / completes) and
    /// bumps `conditional_depth` for the duration of the body.
    /// Body recursion deferred to **C41f1**.
    pub fn handle_catch_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if cmd_name != "catch" || args.is_empty() {
            return false;
        }
        // Result var (args[1]) and options var (args[2]).
        for (i, name) in args.iter().enumerate().take(3).skip(1) {
            if let Some(tok) = arg_tokens.get(i) {
                self.define_var(name, *tok, scope_path, false, None);
            }
        }
        true
    }

    /// Handle `try BODY ?on/trap CODE VARLIST BODY?... ?finally BODY?`.
    ///
    /// Mirrors `_handle_try_command` in
    /// `core/analysis/_analyser/_handlers.py:200-213`. Defers the
    /// arm-body walk to ``handle_try`` (proc-scope variant in
    /// ``_proc.py:333-359``) which lands in **C41c3**. The entry
    /// shim only validates the canonical shape; arity checking
    /// lives in `compiler_checks::arity_checks` already.
    pub fn handle_try_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        _arg_tokens: &[Token],
        _scope_path: &[usize],
    ) -> bool {
        if cmd_name != "try" || args.is_empty() {
            return false;
        }
        // C41c3: delegate to handle_try for arm body walk.
        true
    }

    /// Handle `interp alias {} ALIAS {} TARGET ?ARG ...?` —
    /// records the alias for later argument-role resolution.
    ///
    /// Mirrors `_handle_interp_alias` in
    /// `core/analysis/_analyser/_handlers.py:225-236`. Delegates
    /// the actual detection logic to `crate::alias::detect_interp_alias`
    /// (which already handles the canonical `interp alias {}`
    /// shape and the `args[5..]` prepended-args slice).
    pub fn handle_interp_alias(&mut self, cmd_name: &str, args: &[String]) {
        let Some((qualified, target_cmd, prepended)) = detect_interp_alias(cmd_name, args) else {
            return;
        };
        self.command_aliases
            .insert(qualified.clone(), (target_cmd.clone(), prepended.clone()));
        self.result.command_aliases.insert(
            qualified.clone(),
            SignatureCommandAlias {
                qualified_name: qualified,
                target: target_cmd,
                extras: prepended,
            },
        );
    }

    /// Handle `oo::objdefine $obj …` — record the object variable
    /// so later W308 (unknown method on object) checks can suppress
    /// false positives from per-instance method extensions.
    ///
    /// Mirrors `_handle_oo_objdefine` in
    /// `core/analysis/_analyser/_handlers.py:238-252`.
    pub fn handle_oo_objdefine(&mut self, cmd_name: &str, args: &[String]) {
        if cmd_name != "oo::objdefine" || args.is_empty() {
            return;
        }
        let mut obj_name = args[0].trim().to_string();
        if let Some(stripped) = obj_name.strip_prefix('$') {
            obj_name = stripped.trim_matches(|c| c == '{' || c == '}').to_string();
        }
        if !obj_name.is_empty() {
            self.objdefined_vars.insert(obj_name);
        }
    }

    /// Resolve a command alias to `(target_cmd, effective_args)`.
    ///
    /// Mirrors `_resolve_alias` in
    /// `core/analysis/_analyser/_handlers.py:270-287`. Returns
    /// `(cmd_name, args)` unchanged if no alias matches; otherwise
    /// returns the target command and the prepended-args + original
    /// args list. Delegates to `crate::alias::resolve_alias` for the
    /// namespace-aware lookup.
    #[must_use]
    pub fn resolve_alias(
        &mut self,
        cmd_name: &str,
        args: &[String],
        scope_path: &[usize],
    ) -> (String, Vec<String>) {
        let ns = self.namespace_from_scope_path(scope_path);
        // The Rust `alias::resolve_alias` accepts `CommandAliasMap`
        // (alias map keyed by qualified alias name) — same shape as
        // `self.command_aliases` already uses.
        if let Some((target_cmd, prepended)) = resolve_alias(cmd_name, &self.command_aliases, &ns) {
            let mut effective: Vec<String> = prepended;
            effective.extend(args.iter().cloned());
            (target_cmd, effective)
        } else {
            (cmd_name.to_string(), args.to_vec())
        }
    }

    /// Handle `oo::class create NAME ?BODY?` — record the class.
    ///
    /// **C41b8 stub.** The full port (method extraction, mixin
    /// handling, body recursion) lands in **C41e1**. For now this
    /// strip records a minimal [`super::types::ClassDef`] in
    /// ``result.all_classes`` so consumers see the class in the
    /// workspace index. Returns `true` when the command shape
    /// matched.
    pub fn handle_oo_class_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if cmd_name != "oo::class" || args.len() < 2 || arg_tokens.len() < 2 {
            return false;
        }
        if args[0] != "create" {
            return false;
        }
        let raw_name = &args[1];
        let ns_prefix = self.namespace_from_scope_path(scope_path);
        let ns_for_qualify = ns_prefix.trim_start_matches(':');
        let qualified = qualify(ns_for_qualify, raw_name);
        let simple = qualified.rsplit("::").next().unwrap_or("").to_string();
        let name_span = arg_tokens[1].span;
        let body_span = arg_tokens.get(2).map_or(arg_tokens[1].span, |t| t.span);
        let class = super::types::ClassDef {
            name: simple,
            qualified_name: qualified.clone(),
            name_span,
            body_span,
        };
        self.result.all_classes.insert(qualified, class);
        true
    }

    /// Handle `oo::define CLASS ?BODY?` — record an extension to
    /// an existing class.
    ///
    /// **C41b8 stub.** The full port (method addition, mixin
    /// declarations, superclass changes) lands in **C41e2**. For
    /// now this strip is a recognise-and-no-op so the dispatch
    /// table can route the command without a fall-through to
    /// W123 unresolved-command later.
    pub fn handle_oo_define_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        _arg_tokens: &[Token],
        _scope_path: &[usize],
    ) -> bool {
        if cmd_name != "oo::define" || args.is_empty() {
            return false;
        }
        // C41e2 will populate the class extension here.
        true
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
    fn handle_proc_keys_scope_procs_by_simple_name() {
        // Mirrors the Python contract in
        // ``core/analysis/_analyser/_proc.py:111`` —
        // ``scope.procs[simple_name] = proc_def`` so per-scope
        // lookups and shadowing rules work locally. The
        // qualified name lives on ``ProcDef.qualified_name`` and
        // is the key for ``result.all_procs``.
        use crate::analyser::types::{Scope, ScopeKind};
        let mut a = Analyser::new();
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "ns1"));
        a.handle_proc_command(
            "proc",
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[0],
        );
        let scope = &a.result.global_scope.children[0];
        // Scope's procs map keyed by simple name, NOT qualified.
        assert!(
            scope.procs.contains_key("foo"),
            "scope.procs should be keyed by simple `foo`, got keys: {:?}",
            scope.procs.keys().collect::<Vec<_>>(),
        );
        assert!(
            !scope.procs.contains_key("::ns1::foo"),
            "scope.procs must not be keyed by qualified name",
        );
        // The qualified name is still on the ProcDef.
        assert_eq!(scope.procs["foo"].qualified_name, "::ns1::foo");
        // ...and result.all_procs is keyed by qualified name.
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

    // -- handle_proc_command body recursion (C41c1) -----------------

    #[test]
    fn handle_proc_creates_proc_scope_for_braced_body() {
        // ``proc foo {} {}`` — empty braced body still opens a
        // proc scope so subsequent body-walking handlers have a
        // place to record locals.
        let mut a = Analyser::new();
        a.handle_proc_command(
            "proc",
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
        );
        assert_eq!(a.result.global_scope.children.len(), 1);
        let proc_scope = &a.result.global_scope.children[0];
        assert_eq!(proc_scope.kind, crate::analyser::types::ScopeKind::Proc);
        assert_eq!(proc_scope.name, "foo");
        assert_eq!(proc_scope.body_span, Some(span(12, 14)));
    }

    #[test]
    fn handle_proc_defines_params_in_proc_scope() {
        // ``proc foo {a b} {}`` — a, b become locals in the
        // proc scope, not in the outer scope.
        let mut a = Analyser::new();
        a.handle_proc_command(
            "proc",
            &["foo".to_string(), "a b".to_string(), String::new()],
            &[
                esc_tok(span(5, 8)),
                esc_tok(span(9, 14)),
                str_tok(span(15, 17)),
            ],
            &[],
        );
        let proc_scope = &a.result.global_scope.children[0];
        assert!(proc_scope.variables.contains_key("a"));
        assert!(proc_scope.variables.contains_key("b"));
        // Outer scope must be untouched.
        assert!(!a.result.global_scope.variables.contains_key("a"));
    }

    #[test]
    fn handle_proc_walks_body_set_defines_local() {
        // ``proc foo {} {set x 1}`` — body walk segments the
        // body and dispatches `set x 1` against the proc scope,
        // landing the local in proc_scope.variables, not global.
        // The body token's span must mirror the outer source so
        // the segmenter rebases correctly: source layout is
        // ``proc foo {} {set x 1}`` with the body occupying [13, 22].
        // ``content_offset = 1`` skips the leading ``{`` so the
        // re-segmented inner runs at base 14.
        let mut a = Analyser::new();
        a.handle_proc_command(
            "proc",
            &["foo".to_string(), String::new(), "set x 1".to_string()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(13, 22)),
            ],
            &[],
        );
        let proc_scope = &a.result.global_scope.children[0];
        assert!(
            proc_scope.variables.contains_key("x"),
            "body walk should land 'x' in proc scope; vars: {:?}",
            proc_scope.variables.keys().collect::<Vec<_>>(),
        );
        assert!(!a.result.global_scope.variables.contains_key("x"));
    }

    #[test]
    fn handle_proc_walks_body_global_falls_into_proc_scope() {
        // ``proc foo {} {global a b}`` — the ``global`` handler
        // defines bindings in the proc scope so the body's later
        // reads/writes resolve correctly. Real ``global`` semantics
        // (link to outer var) live with diagnostic emission later.
        let mut a = Analyser::new();
        a.handle_proc_command(
            "proc",
            &["foo".to_string(), String::new(), "global a b".to_string()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(13, 25)),
            ],
            &[],
        );
        let proc_scope = &a.result.global_scope.children[0];
        assert!(proc_scope.variables.contains_key("a"));
        assert!(proc_scope.variables.contains_key("b"));
    }

    #[test]
    fn handle_proc_nested_proc_creates_nested_scopes() {
        // Body-walk recursion must dispatch `proc` inside the body,
        // creating a nested proc scope under the outer proc.
        let mut a = Analyser::new();
        a.handle_proc_command(
            "proc",
            &[
                "outer".to_string(),
                String::new(),
                "proc inner {} {}".to_string(),
            ],
            &[
                esc_tok(span(5, 10)),
                str_tok(span(11, 13)),
                str_tok(span(15, 33)),
            ],
            &[],
        );
        // Outer proc registered.
        assert!(a.result.all_procs.contains_key("::outer"));
        // Inner proc registered under outer's qualified prefix? In
        // Python, ``namespace_from_scope_path`` skips proc scopes —
        // so an `inner` proc declared inside ``outer`` qualifies as
        // ``::inner`` (the outer proc is not a namespace). Match
        // that contract here.
        assert!(a.result.all_procs.contains_key("::inner"));
        // Outer's proc scope holds the nested proc scope as a child.
        let outer_scope = &a.result.global_scope.children[0];
        assert!(!outer_scope.children.is_empty());
        assert_eq!(
            outer_scope.children[0].kind,
            crate::analyser::types::ScopeKind::Proc,
        );
        assert_eq!(outer_scope.children[0].name, "inner");
    }

    #[test]
    fn handle_proc_dynamic_body_skips_walk() {
        // ``proc foo {} $body`` — the body is a Var token, not a
        // Str token; we cannot statically re-segment a dynamic
        // body, so the body walk is skipped. The proc record
        // itself still lands so downstream signature consumers see
        // the proc; only the inner walk is gated.
        let mut a = Analyser::new();
        let var_tok = Token::new(TokenType::Var, span(13, 18));
        a.handle_proc_command(
            "proc",
            &["foo".to_string(), String::new(), "$body".to_string()],
            &[esc_tok(span(5, 8)), str_tok(span(9, 11)), var_tok],
            &[],
        );
        assert!(a.result.all_procs.contains_key("::foo"));
        // No proc scope opened — Str gate failed.
        assert!(a.result.global_scope.children.is_empty());
    }

    #[test]
    fn handle_proc_body_walk_increments_body_depth_temporarily() {
        // ``body_depth`` is bumped for the duration of the body
        // walk and restored on exit — top-level-only command
        // checks (C41d) rely on the depth being zero outside any
        // body.
        let mut a = Analyser::new();
        assert_eq!(a.body_depth, 0);
        a.handle_proc_command(
            "proc",
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
        );
        assert_eq!(a.body_depth, 0);
    }

    #[test]
    fn handle_proc_body_walk_does_not_leak_inner_doc_comment() {
        // A trailing comment inside the body should not bleed into
        // ``last_comment`` for whatever follows the proc at the
        // outer scope. The outer comment ("doc string") is consumed
        // as the proc's own doc; after the walk, ``last_comment``
        // is restored to empty.
        let mut a = Analyser::new();
        a.last_comment = "doc string".to_string();
        a.handle_proc_command(
            "proc",
            &["foo".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
        );
        assert_eq!(a.result.all_procs["::foo"].doc, "doc string");
        assert!(a.last_comment.is_empty());
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

    // -- handle_foreach_command -------------------------------------

    #[test]
    fn handle_foreach_defines_single_loop_var() {
        let mut a = Analyser::new();
        let handled = a.handle_foreach_command(
            "foreach",
            &[
                "i".to_string(),
                "{1 2 3}".to_string(),
                "puts $i".to_string(),
            ],
            &[
                esc_tok(span(8, 9)),
                str_tok(span(10, 17)),
                str_tok(span(18, 28)),
            ],
            &[],
        );
        assert!(handled);
        assert!(a.result.global_scope.variables.contains_key("i"));
    }

    #[test]
    fn handle_foreach_defines_multiple_loop_vars() {
        let mut a = Analyser::new();
        a.handle_foreach_command(
            "foreach",
            &["k v".to_string(), "{a 1 b 2}".to_string(), String::new()],
            &[
                esc_tok(span(8, 11)),
                str_tok(span(12, 21)),
                str_tok(span(22, 24)),
            ],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("k"));
        assert!(a.result.global_scope.variables.contains_key("v"));
    }

    #[test]
    fn handle_foreach_too_few_args_returns_false() {
        let mut a = Analyser::new();
        let handled = a.handle_foreach_command(
            "foreach",
            &["i".to_string(), "{1 2}".to_string()],
            &[esc_tok(span(0, 1)), str_tok(span(2, 7))],
            &[],
        );
        assert!(!handled);
    }

    #[test]
    fn handle_foreach_wrong_command_returns_false() {
        let mut a = Analyser::new();
        let handled = a.handle_foreach_command(
            "while",
            &["i".to_string(), "list".to_string(), "body".to_string()],
            &[
                esc_tok(span(0, 1)),
                esc_tok(span(2, 6)),
                esc_tok(span(7, 11)),
            ],
            &[],
        );
        assert!(!handled);
    }

    // -- handle_for_command -----------------------------------------

    #[test]
    fn handle_for_returns_true_for_canonical_shape() {
        let mut a = Analyser::new();
        let handled = a.handle_for_command(
            "for",
            &[
                "set i 0".to_string(),
                "$i < 10".to_string(),
                "incr i".to_string(),
                "puts $i".to_string(),
            ],
            &[],
            &[],
        );
        assert!(handled);
    }

    #[test]
    fn handle_for_too_few_args_returns_false() {
        let mut a = Analyser::new();
        let handled = a.handle_for_command(
            "for",
            &["set i 0".to_string(), "$i < 10".to_string()],
            &[],
            &[],
        );
        assert!(!handled);
    }

    // -- handle_switch_command --------------------------------------

    #[test]
    fn handle_switch_returns_true_for_canonical_shape() {
        let mut a = Analyser::new();
        let handled = a.handle_switch_command(
            "switch",
            &["$x".to_string(), "{a {puts a} b {puts b}}".to_string()],
            &[],
            &[],
        );
        assert!(handled);
    }

    #[test]
    fn handle_switch_too_few_args_returns_false() {
        let mut a = Analyser::new();
        let handled = a.handle_switch_command("switch", &["$x".to_string()], &[], &[]);
        assert!(!handled);
    }

    // -- handle_catch_command ---------------------------------------

    #[test]
    fn handle_catch_canonical_returns_true() {
        let mut a = Analyser::new();
        let handled =
            a.handle_catch_command("catch", &["body".to_string()], &[esc_tok(span(0, 4))], &[]);
        assert!(handled);
    }

    #[test]
    fn handle_catch_with_result_var_defines_it() {
        let mut a = Analyser::new();
        a.handle_catch_command(
            "catch",
            &["body".to_string(), "res".to_string()],
            &[esc_tok(span(0, 4)), esc_tok(span(5, 8))],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("res"));
    }

    #[test]
    fn handle_catch_with_options_var_defines_both() {
        let mut a = Analyser::new();
        a.handle_catch_command(
            "catch",
            &["body".to_string(), "res".to_string(), "opts".to_string()],
            &[
                esc_tok(span(0, 4)),
                esc_tok(span(5, 8)),
                esc_tok(span(9, 13)),
            ],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("res"));
        assert!(a.result.global_scope.variables.contains_key("opts"));
    }

    #[test]
    fn handle_catch_no_args_returns_false() {
        let mut a = Analyser::new();
        let handled = a.handle_catch_command("catch", &[], &[], &[]);
        assert!(!handled);
    }

    // -- handle_try_command -----------------------------------------

    #[test]
    fn handle_try_canonical_returns_true() {
        let mut a = Analyser::new();
        let handled = a.handle_try_command("try", &["body".to_string()], &[], &[]);
        assert!(handled);
    }

    #[test]
    fn handle_try_no_args_returns_false() {
        let mut a = Analyser::new();
        let handled = a.handle_try_command("try", &[], &[], &[]);
        assert!(!handled);
    }

    // -- handle_interp_alias ----------------------------------------

    #[test]
    fn handle_interp_alias_records_canonical_form() {
        let mut a = Analyser::new();
        a.handle_interp_alias(
            "interp",
            &[
                "alias".to_string(),
                String::new(),
                "myset".to_string(),
                String::new(),
                "set".to_string(),
            ],
        );
        assert!(a.command_aliases.contains_key("::myset"));
        assert!(a.result.command_aliases.contains_key("::myset"));
        let (target, prepended) = &a.command_aliases["::myset"];
        assert_eq!(target, "set");
        assert!(prepended.is_empty());
    }

    #[test]
    fn handle_interp_alias_with_prepended_args() {
        let mut a = Analyser::new();
        a.handle_interp_alias(
            "interp",
            &[
                "alias".to_string(),
                String::new(),
                "logerr".to_string(),
                String::new(),
                "puts".to_string(),
                "stderr".to_string(),
            ],
        );
        let (target, prepended) = &a.command_aliases["::logerr"];
        assert_eq!(target, "puts");
        assert_eq!(prepended, &vec!["stderr".to_string()]);
    }

    #[test]
    fn handle_interp_alias_wrong_shape_no_op() {
        let mut a = Analyser::new();
        a.handle_interp_alias("interp", &["alias".to_string()]);
        assert!(a.command_aliases.is_empty());
    }

    // -- handle_oo_objdefine ----------------------------------------

    #[test]
    fn handle_oo_objdefine_records_dollar_var() {
        let mut a = Analyser::new();
        a.handle_oo_objdefine("oo::objdefine", &["$obj".to_string()]);
        assert!(a.objdefined_vars.contains("obj"));
    }

    #[test]
    fn handle_oo_objdefine_records_braced_dollar_var() {
        let mut a = Analyser::new();
        a.handle_oo_objdefine("oo::objdefine", &["${obj}".to_string()]);
        assert!(a.objdefined_vars.contains("obj"));
    }

    #[test]
    fn handle_oo_objdefine_records_bare_name() {
        let mut a = Analyser::new();
        a.handle_oo_objdefine("oo::objdefine", &["obj".to_string()]);
        assert!(a.objdefined_vars.contains("obj"));
    }

    #[test]
    fn handle_oo_objdefine_wrong_command_no_op() {
        let mut a = Analyser::new();
        a.handle_oo_objdefine("oo::class", &["$obj".to_string()]);
        assert!(a.objdefined_vars.is_empty());
    }

    // -- resolve_alias ----------------------------------------------

    #[test]
    fn resolve_alias_passthrough_for_non_alias() {
        let mut a = Analyser::new();
        let (target, args) = a.resolve_alias("puts", &["hello".to_string()], &[]);
        assert_eq!(target, "puts");
        assert_eq!(args, vec!["hello".to_string()]);
    }

    #[test]
    fn resolve_alias_substitutes_target_and_prepended_args() {
        let mut a = Analyser::new();
        a.command_aliases.insert(
            "::logerr".to_string(),
            ("puts".to_string(), vec!["stderr".to_string()]),
        );
        let (target, args) = a.resolve_alias("logerr", &["hello".to_string()], &[]);
        assert_eq!(target, "puts");
        assert_eq!(args, vec!["stderr".to_string(), "hello".to_string()]);
    }

    // -- handle_oo_class_command (C41b8 stub) -----------------------

    #[test]
    fn handle_oo_class_create_records_class() {
        let mut a = Analyser::new();
        let handled = a.handle_oo_class_command(
            "oo::class",
            &["create".to_string(), "MyClass".to_string()],
            &[
                esc_tok(span(0, 9)),
                esc_tok(span(10, 16)),
                esc_tok(span(17, 24)),
            ],
            &[],
        );
        assert!(handled);
        assert!(a.result.all_classes.contains_key("::MyClass"));
        let cls = &a.result.all_classes["::MyClass"];
        assert_eq!(cls.name, "MyClass");
    }

    #[test]
    fn handle_oo_class_create_with_body() {
        // arg_tokens stripped of cmd_name (matching the
        // ``process_command`` dispatch convention).
        let mut a = Analyser::new();
        let handled = a.handle_oo_class_command(
            "oo::class",
            &[
                "create".to_string(),
                "MyClass".to_string(),
                "method m {} {}".to_string(),
            ],
            &[
                esc_tok(span(10, 16)),
                esc_tok(span(17, 24)),
                str_tok(span(25, 41)),
            ],
            &[],
        );
        assert!(handled);
        assert_eq!(a.result.all_classes["::MyClass"].body_span, span(25, 41));
    }

    #[test]
    fn handle_oo_class_wrong_subcommand_returns_false() {
        let mut a = Analyser::new();
        let handled = a.handle_oo_class_command(
            "oo::class",
            &["destroy".to_string(), "MyClass".to_string()],
            &[
                esc_tok(span(0, 9)),
                esc_tok(span(10, 17)),
                esc_tok(span(18, 25)),
            ],
            &[],
        );
        assert!(!handled);
        assert!(a.result.all_classes.is_empty());
    }

    // -- handle_oo_define_command (C41b8 stub) ----------------------

    #[test]
    fn handle_oo_define_recognises_canonical_form() {
        let mut a = Analyser::new();
        let handled = a.handle_oo_define_command(
            "oo::define",
            &["MyClass".to_string(), "method m {} {}".to_string()],
            &[],
            &[],
        );
        assert!(handled);
    }

    #[test]
    fn handle_oo_define_no_args_returns_false() {
        let mut a = Analyser::new();
        let handled = a.handle_oo_define_command("oo::define", &[], &[], &[]);
        assert!(!handled);
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
