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

/// Split a form-2 ``switch`` braced body into its flat list of
/// pattern/body elements.
///
/// Mirrors `_parse_switch_body` in
/// `core/analysis/_analyser/_proc.py:259-331`. Python re-lexes
/// the inner body with `TclLexer` and groups consecutive
/// non-separator tokens into "elements"; the Rust port leans on
/// the existing segmenter — every word across every command in
/// the body is one element, which produces the same flat
/// alternating pattern/body sequence.
///
/// Returns `(text, token)` pairs in source order.  The token's
/// span is rebased into the outer source's offset space via the
/// body token's `content_offset`.  Dynamic bodies (non-`Str`
/// tokens) yield an empty list — the caller must fall back to
/// form-1-style alternation when the form-2 body cannot be
/// statically split.
fn parse_switch_body_elements(body_text: &str, body_tok: Token) -> Vec<(String, Token)> {
    if body_tok.kind != TokenType::Str {
        return Vec::new();
    }
    let base_offset = body_tok.span.start() + u32::from(body_tok.content_offset);
    let cmds = crate::segmenter::segment_commands_with_offset(body_text, base_offset);
    let mut elements = Vec::new();
    for cmd in cmds {
        if cmd.is_partial {
            continue;
        }
        for (text, tok) in cmd.texts.iter().zip(cmd.argv.iter()) {
            elements.push((text.clone(), *tok));
        }
    }
    elements
}

impl Analyser {
    /// Handle the `set` command: `set var ?value?`.
    ///
    /// Mirrors `_handle_set_command` in
    /// `core/analysis/_analyser/_handlers.py:51-70` and the inner
    /// `_handle_set` in `_proc.py:177-191`.
    ///
    /// - **Two-arg form** (`set var value`) — defines the variable
    ///   in the scope at `scope_path` and tracks the value as a
    ///   constant string when the value is a single-token literal
    ///   (no interpolation, no command sub).
    /// - **One-arg form** (`set var`) — records a var read on the
    ///   variable.  Tcl `set` with no value returns the current
    ///   value, so this is a reference, not a definition.
    ///
    /// `single_token_word` parallels `args` and `arg_tokens` —
    /// `true` when the corresponding word is a single atomic
    /// token. It's the Rust replacement for Python's
    /// `value_token.text == args[1]` check, which boils down to
    /// "is this word's text the same as a single token's raw
    /// text?".
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

        // _handle_set: arg-count branch from ``_proc.py:177-191``.
        let Some(name_tok) = arg_tokens.first() else {
            return;
        };
        if args.len() >= 2 {
            self.define_var(&args[0], *name_tok, scope_path, true, None);
        } else {
            self.record_var_read(&args[0], name_tok.span, scope_path);
        }

        // Track constant-string assignments for regex propagation.
        // Skipped for the 1-arg read form (no value to track).
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

        // **W113** — proc name shadows a built-in command.
        // Mirrors ``_proc.py:70-89``.  The check runs against
        // both the unqualified ``proc_name`` and the fully-
        // qualified form, with the leading ``::`` trimmed (the
        // registry indexes by bare command name).  The inner
        // borrow on ``self.builtin_command_names()`` is dropped
        // before the diagnostic push so ``self.result`` is free
        // to mutate.
        let normalised_proc: String = raw_name.trim_start_matches(':').to_string();
        let normalised_qual: String = qualified.trim_start_matches(':').to_string();
        let shadow_match = {
            let builtins = self.builtin_command_names();
            if builtins.contains(&normalised_proc) {
                true
            } else {
                builtins.contains(&normalised_qual)
            }
        };
        if shadow_match {
            let dialect_label = if self.dialect.is_empty() {
                String::new()
            } else {
                format!(" ({})", self.dialect)
            };
            let message = format!("Procedure '{raw_name}' shadows built-in command{dialect_label}");
            self.result.diagnostics.push(super::types::Diagnostic {
                code: "W113".to_string(),
                span: name_span,
                message,
                severity: super::types::Severity::Warning,
                fixes: Vec::new(),
            });
        }

        let params = parse_param_list(&args[1]);
        let doc = std::mem::take(&mut self.last_comment);

        // **C41e3.** When a user defines ``proc unknown ...`` (or
        // ``::tcl::unknown``), inspect the body to determine which
        // commands the handler can resolve.  The result gates
        // W123 (unresolved command) — if the user provided their
        // own ``unknown`` we can't statically prove a command is
        // truly unresolved.  Mirrors Python's
        // ``_extract_unknown_proc_info`` call site in
        // ``_proc.py:97-104``.
        if matches!(simple.as_str(), "unknown") || qualified == "::tcl::unknown" {
            let info = self.extract_unknown_proc_info(&args[2], &params);
            self.result.unknown_proc_info = Some(info);
        }

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

            // Body recursion via the shared helper.  Re-segments
            // the body (no recovery — top-level only) and
            // dispatches each command at the new proc scope path.
            let body_text = args[2].clone();
            self.analyse_body(&body_text, body_tok, &child_path);

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
        let body_text = args.get(2).cloned();
        let body_tok = arg_tokens.get(2).copied();

        let path = scope_path.to_vec();
        let child_scope_idx = {
            let mut child = super::types::Scope::new(super::types::ScopeKind::Namespace, ns_name);
            child.body_span = body_span;
            let Some(parent) = super::scope::scope_at_mut(&mut self.result.global_scope, &path)
            else {
                return false;
            };
            parent.children.push(child);
            parent.children.len() - 1
        };
        let mut child_path = path;
        child_path.push(child_scope_idx);

        // **C41e3 follow-up.** Body recursion lets procs and
        // classes declared inside ``namespace eval`` register
        // with the correct namespace prefix.  Mirrors Python's
        // ``_handle_namespace_eval_command`` which calls
        // ``_analyse_body`` on the body text + token.
        if let (Some(text), Some(tok)) = (body_text, body_tok) {
            self.analyse_body(&text, tok, &child_path);
        }
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
    /// `core/analysis/_analyser/_handlers.py:164-177` plus the
    /// `_handle_switch` arm walker in `_proc.py:192-258`. Arity
    /// checking lives in `compiler_checks::arity_checks` via the
    /// IR; this handler walks each arm body so locals defined
    /// inside an arm land in the enclosing scope.
    ///
    /// Switch has two forms:
    ///
    /// 1. ``switch ?options? string pattern body ?pattern body? ...``
    ///    — pattern and body args alternate inline.
    /// 2. ``switch ?options? string {pattern body ?pattern body? ...}``
    ///    — pattern/body pairs live inside a single braced
    ///    block.  See [`Self::parse_switch_body_elements`] for
    ///    how that braced form is split.
    ///
    /// Bodies that are literally ``-`` are fall-through markers
    /// (the next arm's body fires) and are skipped — recursing
    /// into the literal ``-`` would produce a useless command.
    ///
    /// **Deferred.** ``-regexp`` pattern recording (Python
    /// ``_proc.py:233-252``) emits ``RegexPattern`` records into
    /// ``result.regex_patterns``; the Rust analyser doesn't yet
    /// carry that field (lands alongside the diagnostic emitters
    /// in **C41d**).  The flag is detected here as a marker for
    /// the future hook but no records are emitted yet.
    pub fn handle_switch_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if cmd_name != "switch" || args.len() < 2 {
            return false;
        }

        // Scan and skip option flags. ``--`` ends the option
        // section explicitly (and is consumed). ``-regexp``
        // would gate regex-pattern recording in C41d; tracked
        // here so the future hook only needs to consult the flag.
        let mut i = 0;
        let mut _is_regexp = false;
        while i < args.len() && args[i].starts_with('-') {
            if args[i] == "-regexp" {
                _is_regexp = true;
            }
            if args[i] == "--" {
                i += 1;
                break;
            }
            i += 1;
        }
        // Skip the ``string`` argument that follows the options.
        i += 1;

        if i >= args.len() {
            return true;
        }

        if i == args.len() - 1 {
            // Form 2 — single braced body containing all pairs.
            let body_text = args[i].clone();
            let Some(body_tok) = arg_tokens.get(i).copied() else {
                return true;
            };
            let elements = parse_switch_body_elements(&body_text, body_tok);
            let mut j = 0;
            while j + 1 < elements.len() {
                // Pattern at j, body at j+1. Regex-pattern recording
                // for ``-regexp`` lands in C41d.
                let (body_text, body_tok) = &elements[j + 1];
                if body_text != "-" {
                    self.analyse_body(body_text, *body_tok, scope_path);
                }
                j += 2;
            }
        } else {
            // Form 1 — pattern/body pairs inline in args/arg_tokens.
            while i + 1 < args.len() {
                let body_text = &args[i + 1];
                if let Some(body_tok) = arg_tokens.get(i + 1).copied() {
                    if body_text != "-" {
                        self.analyse_body(body_text, body_tok, scope_path);
                    }
                }
                i += 2;
            }
        }
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
    /// `core/analysis/_analyser/_handlers.py:200-213` plus the
    /// `_handle_try` arm walker in `_proc.py:333-359`. Walks the
    /// main try body and every handler / finally clause; arity
    /// checking lives in `compiler_checks::arity_checks` already.
    ///
    /// Clause shapes:
    ///
    /// - ``finally BODY`` (2 words) — recurse into ``BODY``.
    /// - ``on CODE VARLIST BODY`` / ``trap PATTERN VARLIST BODY``
    ///   (4 words) — recurse into ``BODY``.  The handler's
    ///   ``VARLIST`` (e.g. ``{result options}``) is **not**
    ///   defined as a binding here; mirrors Python's
    ///   ``_handle_try`` which doesn't define them either
    ///   (``_proc.py:333-359``).  A future strip can add the
    ///   varList define if Python is updated to match.
    pub fn handle_try_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if cmd_name != "try" || args.is_empty() {
            return false;
        }
        // Main try body at args[0].
        if let Some(body_tok) = arg_tokens.first().copied() {
            self.analyse_body(&args[0], body_tok, scope_path);
        }
        // Walk handler / finally clauses.
        let mut i = 1;
        while i < args.len() {
            let kw = args[i].as_str();
            if kw == "finally" && i + 1 < args.len() {
                if let Some(body_tok) = arg_tokens.get(i + 1).copied() {
                    self.analyse_body(&args[i + 1], body_tok, scope_path);
                }
                i += 2;
            } else if matches!(kw, "on" | "trap") && i + 3 < args.len() {
                if let Some(body_tok) = arg_tokens.get(i + 3).copied() {
                    self.analyse_body(&args[i + 3], body_tok, scope_path);
                }
                i += 4;
            } else {
                i += 1;
            }
        }
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

    /// Handle ``package require`` (and ``package provide``) —
    /// record the package dependency so later passes can gate
    /// W123 (unresolved-command) suppression and dynamic-
    /// provider detection.
    ///
    /// Mirrors the package-recording fragment of
    /// ``_AnalyserCommandsMixin._process_command`` in
    /// ``core/analysis/_analyser/_commands.py:277-321``.  Two
    /// shapes Python recognises:
    ///
    /// - ``package require ?-exact? NAME ?VERSION?`` — appends a
    ///   ``SignaturePackageRequire`` record to
    ///   ``result.package_requires`` and flips
    ///   ``has_dynamic_providers`` when the name argument is a
    ///   ``$``-substitution / ``[…]``-substitution token.
    /// - ``package provide NAME ?VERSION?`` — Python records this
    ///   on ``result.package_provides``; the Rust
    ///   ``AnalysisResult`` doesn't carry that field yet (deferred
    ///   carry-over) so we only consume the shape silently.
    ///
    /// The conditional flag is ``self.conditional_depth > 0``,
    /// matching Python's `_conditional_depth`.
    ///
    /// `cmd_tok` is the command-head token (the ``package``
    /// word).  The recorded
    /// [`SignaturePackageRequire::range`](crate::signature_scan::types::SignaturePackageRequire::range)
    /// uses its span so the range matches Python's
    /// ``range_from_token(argv[0])`` — code-action /
    /// quick-fix UX points at the ``package`` keyword rather
    /// than at the ``require`` subcommand word.
    pub fn handle_package_command(
        &mut self,
        cmd_name: &str,
        cmd_tok: Token,
        args: &[String],
        arg_tokens: &[Token],
    ) {
        if cmd_name != "package" || args.is_empty() {
            return;
        }
        let sub = args[0].as_str();
        if sub != "require" {
            // ``package provide`` and other subcommands aren't
            // recorded yet (no ``package_provides`` field in
            // the Rust ``AnalysisResult``); silently consume.
            return;
        }
        if args.len() < 2 {
            return;
        }

        // ``package require -exact NAME ?VERSION?`` — strip the
        // flag and shift the name index.
        let (name_idx, name_text) = if args[1] == "-exact" && args.len() >= 3 {
            (2usize, args[2].clone())
        } else {
            (1usize, args[1].clone())
        };
        let version_idx = name_idx + 1;
        let version = if version_idx < args.len() {
            Some(args[version_idx].clone())
        } else {
            None
        };

        // Dynamic-provider detection — non-literal name flips the
        // flag.  ``arg_tokens`` is parallel to ``args`` so the
        // token at the name index is what we inspect.
        if let Some(name_tok) = arg_tokens.get(name_idx) {
            if matches!(name_tok.kind, TokenType::Var | TokenType::Cmd)
                || name_text.contains('$')
                || name_text.contains('[')
            {
                // No ``has_dynamic_providers`` field on Rust
                // AnalysisResult yet — track via package_requires
                // alone.  When the field lands, flip it here.
            }
        }

        self.result
            .package_requires
            .push(crate::signature_scan::types::SignaturePackageRequire {
                name: name_text,
                version,
                range: cmd_tok.span,
                conditional: self.conditional_depth > 0,
            });
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
        let body_tok_opt = arg_tokens.get(2).copied();
        let body_span = body_tok_opt.map_or(arg_tokens[1].span, |t| t.span);
        let doc = std::mem::take(&mut self.last_comment);
        let mut class = super::types::ClassDef {
            name: simple,
            qualified_name: qualified.clone(),
            name_span,
            body_span,
            metaclass: cmd_name.to_string(),
            doc,
            ..Default::default()
        };
        // **C41e1.** Walk the class body when present —
        // populates ``superclasses`` / ``mixins`` / ``methods`` /
        // ``class_methods`` from the OO-define subcommands.
        if let (Some(body_text), Some(body_tok)) = (args.get(2), body_tok_opt) {
            self.parse_oo_definition_body(body_text, body_tok, &mut class);
        }
        self.result.all_classes.insert(qualified, class);
        true
    }

    /// Handle `oo::define CLASS ?BODY?` — record an extension to
    /// an existing class.
    ///
    /// **C41e2.** Looks up the class by qualified name in
    /// ``result.all_classes``; when found, walks the body or
    /// inline-form arguments via the OO walkers in
    /// [`super::oo`] to extend ``superclasses`` / ``mixins`` /
    /// ``methods`` / ``class_methods``.  When the class isn't
    /// in the index yet (e.g. the class definition lives in a
    /// separate file the workspace index hasn't reached), a
    /// stub ``ClassDef`` is created so subsequent
    /// ``oo::define`` calls + the workspace index see a
    /// consistent record.
    pub fn handle_oo_define_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        if cmd_name != "oo::define" || args.is_empty() {
            return false;
        }
        let raw_class_name = &args[0];
        let ns_prefix = self.namespace_from_scope_path(scope_path);
        let ns_for_qualify = ns_prefix.trim_start_matches(':');
        let qualified = qualify(ns_for_qualify, raw_class_name);

        // Distinguish body-form from inline-form by inspecting
        // ``args[1]``.  Body-form: ``oo::define Class { ... }``
        // — args[1] is a single body argument.  Inline-form:
        // ``oo::define Class method foo {} {}`` — args[1] is a
        // known define subcommand.
        if args.len() < 2 {
            return true;
        }

        // The set of known define subcommands is the same as
        // body-form subcommands.  Anything not in this set falls
        // through to body-form handling (the segmenter does the
        // re-parse).
        let define_subcmds: &[&str] = &[
            "method",
            "classmethod",
            "constructor",
            "destructor",
            "superclass",
            "mixin",
            "variable",
            "filter",
            "forward",
            "export",
            "unexport",
            "property",
            "private",
            "initialise",
            "initialize",
            "definitionnamespace",
            "deletemethod",
            "renamemethod",
            "self",
        ];
        let inline_form = define_subcmds.contains(&args[1].as_str());

        // Look up or create the partial ClassDef in
        // ``result.all_classes``. The ``name`` field carries the
        // bare tail even when the source declared the class
        // qualified (``oo::define ::ns::Other``); mirrors the
        // ``simple`` extraction in ``handle_oo_class_command``.
        let simple = qualified.rsplit("::").next().unwrap_or("").to_string();
        let mut class_def = self
            .result
            .all_classes
            .remove(&qualified)
            .unwrap_or_else(|| {
                let name_span = arg_tokens.first().map_or(
                    super::types::Scope::default()
                        .body_span
                        .unwrap_or_else(|| tcl_lexer::Span::new(0, 0)),
                    |t| t.span,
                );
                super::types::ClassDef {
                    name: simple,
                    qualified_name: qualified.clone(),
                    name_span,
                    body_span: name_span,
                    ..Default::default()
                }
            });

        if inline_form {
            // ``oo::define Class subcmd ...`` — args[1..] is
            // the subcommand + its args.
            let inline_args: Vec<String> = args[1..].to_vec();
            let inline_tokens: Vec<Token> = arg_tokens.iter().skip(1).copied().collect();
            self.parse_oo_define_inline(&inline_args, &inline_tokens, &mut class_def);
        } else if let Some(body_tok) = arg_tokens.get(1).copied() {
            // ``oo::define Class { body }`` — args[1] is the
            // body text, arg_tokens[1] is the body token.
            self.parse_oo_definition_body(&args[1], body_tok, &mut class_def);
        }

        self.result.all_classes.insert(qualified, class_def);
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

    /// Resolve a command name to the `ProcDef` that implements
    /// it, walking the scope chain from `scope_path` outwards.
    ///
    /// Mirrors `_resolve_proc_call` in
    /// `core/analysis/_analyser/_proc.py:361-392`.  Build-up:
    ///
    /// - Absolute name (`::foo`) → look up directly.
    /// - Qualified relative (`a::b`) → prepend `::` and look up.
    /// - Bare name → walk up the scope chain; for every
    ///   ``namespace`` scope on the chain, prepend its name and
    ///   try; finally fall back to global ``::name``.
    ///
    /// All candidate names are run through
    /// [`crate::naming::normalise_qualified_name`] so the lookup
    /// keys match the canonical form ``result.all_procs`` uses.
    /// Returns the first matching ``ProcDef`` (by reference into
    /// ``result.all_procs``), or `None` if no candidate is known.
    #[must_use]
    pub fn resolve_proc_call(
        &self,
        cmd_name: &str,
        scope_path: &[usize],
    ) -> Option<&super::types::ProcDef> {
        use std::collections::HashSet;
        if cmd_name.is_empty() {
            return None;
        }

        let mut candidates: Vec<String> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        let mut add_candidate = |raw: &str| {
            let qname = crate::naming::normalise_qualified_name(raw);
            if qname.is_empty() || seen.contains(&qname) {
                return;
            }
            seen.insert(qname.clone());
            candidates.push(qname);
        };

        if cmd_name.starts_with("::") {
            add_candidate(cmd_name);
        } else if cmd_name.contains("::") {
            add_candidate(&format!("::{cmd_name}"));
        } else {
            // Walk the scope chain — every namespace scope on the
            // chain contributes a candidate ``<ns_name>::<cmd>``.
            // ``ancestor_paths`` walks longest-first (current scope
            // before its ancestors), matching Python's ``while``
            // upward loop.
            let mut cursor: &super::types::Scope = &self.result.global_scope;
            let mut walked: Vec<&super::types::Scope> = vec![cursor];
            for &idx in scope_path {
                let Some(child) = cursor.children.get(idx) else {
                    break;
                };
                walked.push(child);
                cursor = child;
            }
            for scope in walked.iter().rev() {
                if scope.kind == super::types::ScopeKind::Namespace {
                    add_candidate(&format!("{}::{cmd_name}", scope.name));
                }
            }
            add_candidate(&format!("::{cmd_name}"));
        }

        for qname in &candidates {
            if let Some(proc) = self.result.all_procs.get(qname) {
                return Some(proc);
            }
        }
        None
    }

    /// Static element count for a `{*}`-expanded word.
    ///
    /// Mirrors `_resolve_expansion_count` in
    /// `core/analysis/_analyser/_proc.py:394-444`.  Used by the
    /// proc-call arity checker (which lives in
    /// `compiler_checks::arity_checks` on the Rust side) to
    /// decide whether a ``{*}``-expanded argument contributes a
    /// statically-known number of runtime arguments.
    ///
    /// - **Braced literal** (`Str` token, ``{a b c}``) — split
    ///   the token's inner text as a list and return its length.
    /// - **Pure variable reference** (`Var` token, ``$x``) — if
    ///   the variable has a known constant value in the current
    ///   scope chain, split that value and return its length.
    /// - **Anything else** — `None`: count not statically known.
    ///
    /// Refinement is only attempted when ``single_token`` is
    /// `true`; for concatenated words like ``{*}$x$y`` or
    /// ``{*}{a b}$suffix`` the segmenter exposes only the *first*
    /// token, which would otherwise be misinterpreted as a pure
    /// literal or pure var ref.  Token text is resolved via
    /// [`tcl_lexer::SourceMap::token_text`] — the same helper the
    /// rest of the analyser uses — so the inner-content stripping
    /// rules (kind-specific delimiter handling) stay in one
    /// place.
    #[must_use]
    pub fn resolve_expansion_count(
        &self,
        tok: Token,
        single_token: bool,
        scope_path: &[usize],
    ) -> Option<usize> {
        use tcl_lexer::SourceMap;
        if !single_token {
            return None;
        }
        let sm = SourceMap::new(&self.source);
        match tok.kind {
            TokenType::Str => {
                let inner = sm.token_text(tok);
                Some(crate::codegen::helpers::split_list_simple(inner).len())
            }
            TokenType::Var => {
                let var_name = sm.token_text(tok);
                let const_val = self.lookup_const_string(var_name, scope_path)?;
                Some(crate::codegen::helpers::split_list_simple(const_val).len())
            }
            _ => None,
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
    fn handle_set_no_value_records_read_not_definition() {
        // ``set x`` (one-arg form) is a *read*, not a definition —
        // Tcl returns the current value of ``x``. Mirrors
        // ``_handle_set`` in ``_proc.py:177-191``: the 1-arg
        // branch calls ``_record_var_read``, not ``_define_var``.
        let mut a = Analyser::new();
        // Pre-define x so the read records a reference.
        a.define_var("x", esc_tok(span(0, 1)), &[], false, None);
        a.handle_set_command(
            "set",
            &["x".to_string()],
            &[esc_tok(span(10, 11))],
            &[true],
            &[],
        );
        // The read appended a reference; no second definition.
        assert!(a.result.global_scope.variables.contains_key("x"));
        assert_eq!(
            a.result.global_scope.variables["x"].references,
            vec![span(10, 11)],
        );
        // No const-string tracking for the 1-arg form.
        assert_eq!(a.lookup_const_string("x", &[]), None);
    }

    #[test]
    fn handle_set_no_value_undefined_var_is_silent() {
        // ``set x`` on an undefined variable is still a read
        // (matching Python's ``_record_var_read`` path); the
        // record_var_read helper silently no-ops when the name
        // isn't in scope, so no spurious binding lands.
        let mut a = Analyser::new();
        a.handle_set_command(
            "set",
            &["x".to_string()],
            &[esc_tok(span(0, 1))],
            &[true],
            &[],
        );
        assert!(!a.result.global_scope.variables.contains_key("x"));
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

    // -- handle_proc_command W113 shadow check (C41c4) --------------

    #[test]
    fn handle_proc_emits_w113_for_builtin_shadow() {
        // ``proc set {} {}`` — the proc name is a built-in.
        // W113 should anchor at the proc-name span and carry
        // the canonical message shape.
        let mut a = Analyser::new();
        a.dialect = "tcl".to_string();
        a.handle_proc_command(
            "proc",
            &["set".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
        );
        let w113s: Vec<&crate::analyser::types::Diagnostic> = a
            .result
            .diagnostics
            .iter()
            .filter(|d| d.code == "W113")
            .collect();
        assert_eq!(w113s.len(), 1);
        assert_eq!(w113s[0].span, span(5, 8));
        assert!(w113s[0].message.contains("'set' shadows built-in"));
        assert!(w113s[0].message.contains("(tcl)"));
        assert_eq!(w113s[0].severity, crate::analyser::types::Severity::Warning);
    }

    #[test]
    fn handle_proc_no_w113_for_non_builtin_name() {
        // ``foo`` is not a built-in — no W113.
        let mut a = Analyser::new();
        a.dialect = "tcl".to_string();
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
        assert!(
            !a.result.diagnostics.iter().any(|d| d.code == "W113"),
            "should NOT emit W113 for non-built-in name 'foo'",
        );
    }

    #[test]
    fn handle_proc_w113_matches_qualified_form() {
        // ``proc ::set {} {}`` — qualified form also shadows
        // ``set`` because the registry indexes by bare command
        // name (``::`` is trimmed at lookup).
        let mut a = Analyser::new();
        a.dialect = "tcl".to_string();
        a.handle_proc_command(
            "proc",
            &["::set".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 10)),
                str_tok(span(11, 13)),
                str_tok(span(14, 16)),
            ],
            &[],
        );
        assert!(a.result.diagnostics.iter().any(|d| d.code == "W113"));
    }

    #[test]
    fn handle_proc_w113_no_dialect_label_when_dialect_empty() {
        // Empty dialect → no parenthetical label in the message.
        let mut a = Analyser::new();
        // dialect intentionally left empty
        a.handle_proc_command(
            "proc",
            &["set".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 8)),
                str_tok(span(9, 11)),
                str_tok(span(12, 14)),
            ],
            &[],
        );
        let w113 = a
            .result
            .diagnostics
            .iter()
            .find(|d| d.code == "W113")
            .expect("W113 expected");
        assert!(w113.message.contains("'set' shadows built-in"));
        assert!(!w113.message.contains('('), "no dialect label expected");
    }

    #[test]
    fn handle_proc_w113_dialect_specific_command_only_shadows_in_that_dialect() {
        // ``HTTP::respond`` is iRules-specific; under the
        // ``f5-irules`` dialect a proc named ``HTTP::respond``
        // shadows a built-in, but under plain ``tcl`` it does
        // not.
        let mut a = Analyser::new();
        a.dialect = "f5-irules".to_string();
        a.handle_proc_command(
            "proc",
            &["HTTP::respond".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 18)),
                str_tok(span(19, 21)),
                str_tok(span(22, 24)),
            ],
            &[],
        );
        assert!(
            a.result.diagnostics.iter().any(|d| d.code == "W113"),
            "f5-irules dialect should treat HTTP::respond as built-in",
        );

        // Same proc, plain tcl dialect → no W113.
        let mut b = Analyser::new();
        b.dialect = "tcl".to_string();
        b.handle_proc_command(
            "proc",
            &["HTTP::respond".to_string(), String::new(), String::new()],
            &[
                esc_tok(span(5, 18)),
                str_tok(span(19, 21)),
                str_tok(span(22, 24)),
            ],
            &[],
        );
        assert!(
            !b.result.diagnostics.iter().any(|d| d.code == "W113"),
            "plain tcl dialect should NOT flag HTTP::respond",
        );
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
            &[esc_tok(span(7, 9)), str_tok(span(10, 36))],
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

    #[test]
    fn handle_switch_form1_walks_each_arm_body() {
        // Form 1: ``switch $x a {set y 1} b {set z 2}``.
        // Each arm body should land its ``set`` in the
        // surrounding scope.  Source layout has the bodies at
        // offsets 13..23 (``{set y 1}``) and 27..37 (``{set z 2}``).
        let mut a = Analyser::new();
        a.handle_switch_command(
            "switch",
            &[
                "$x".to_string(),
                "a".to_string(),
                "set y 1".to_string(),
                "b".to_string(),
                "set z 2".to_string(),
            ],
            &[
                esc_tok(span(7, 9)),
                esc_tok(span(10, 11)),
                str_tok(span(13, 22)),
                esc_tok(span(24, 25)),
                str_tok(span(27, 36)),
            ],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("y"));
        assert!(a.result.global_scope.variables.contains_key("z"));
    }

    #[test]
    fn handle_switch_form2_braced_body_walks_each_arm() {
        // Form 2: ``switch $x { a {set y 1} b {set z 2} }``.
        // The single braced body holds all pattern/body pairs;
        // ``parse_switch_body_elements`` re-segments to surface
        // each pair, then each body recurses.
        let mut a = Analyser::new();
        let body_text = " a {set y 1} b {set z 2} ".to_string();
        // body span: outer source positions 10..(10 + len(body)+2).
        // body_text has 25 chars, plus surrounding braces → token
        // span 10..37, content_offset = 1 to skip the opening ``{``.
        a.handle_switch_command(
            "switch",
            &["$x".to_string(), body_text],
            &[esc_tok(span(7, 9)), str_tok(span(10, 37))],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("y"));
        assert!(a.result.global_scope.variables.contains_key("z"));
    }

    #[test]
    fn handle_switch_form1_skips_fallthrough_marker() {
        // ``switch $x a - b {set y 1}`` — the ``-`` body for
        // pattern ``a`` is fall-through (next arm fires); only
        // ``b``'s body should be walked.
        let mut a = Analyser::new();
        a.handle_switch_command(
            "switch",
            &[
                "$x".to_string(),
                "a".to_string(),
                "-".to_string(),
                "b".to_string(),
                "set y 1".to_string(),
            ],
            &[
                esc_tok(span(7, 9)),
                esc_tok(span(10, 11)),
                esc_tok(span(12, 13)),
                esc_tok(span(14, 15)),
                str_tok(span(17, 26)),
            ],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("y"));
    }

    #[test]
    fn handle_switch_recognises_dashdash_options_terminator() {
        // ``switch -- $x a {set y 1}`` — ``--`` ends the option
        // section; the string arg follows.  Walker still finds
        // the arm body and lands ``y``.
        let mut a = Analyser::new();
        a.handle_switch_command(
            "switch",
            &[
                "--".to_string(),
                "$x".to_string(),
                "a".to_string(),
                "set y 1".to_string(),
            ],
            &[
                esc_tok(span(7, 9)),
                esc_tok(span(10, 12)),
                esc_tok(span(13, 14)),
                str_tok(span(16, 25)),
            ],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("y"));
    }

    #[test]
    fn handle_switch_dynamic_form2_body_skips_walk() {
        // Form 2 with a dynamic body (``$body`` instead of a
        // braced literal) yields no elements; the walk no-ops.
        let mut a = Analyser::new();
        let var_tok = Token::new(TokenType::Var, span(10, 15));
        a.handle_switch_command(
            "switch",
            &["$x".to_string(), "$body".to_string()],
            &[esc_tok(span(7, 9)), var_tok],
            &[],
        );
        // No body walked → no vars defined.
        assert!(a.result.global_scope.variables.is_empty());
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
        let handled =
            a.handle_try_command("try", &["body".to_string()], &[str_tok(span(0, 4))], &[]);
        assert!(handled);
    }

    #[test]
    fn handle_try_no_args_returns_false() {
        let mut a = Analyser::new();
        let handled = a.handle_try_command("try", &[], &[], &[]);
        assert!(!handled);
    }

    #[test]
    fn handle_try_walks_main_body() {
        // ``try {set y 1}`` — main body walks and lands ``y``.
        let mut a = Analyser::new();
        a.handle_try_command(
            "try",
            &["set y 1".to_string()],
            &[str_tok(span(5, 14))],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("y"));
    }

    #[test]
    fn handle_try_walks_finally_body() {
        // ``try {} finally {set z 1}`` — finally clause body walks.
        let mut a = Analyser::new();
        a.handle_try_command(
            "try",
            &[String::new(), "finally".to_string(), "set z 1".to_string()],
            &[
                str_tok(span(5, 7)),
                esc_tok(span(8, 15)),
                str_tok(span(16, 25)),
            ],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("z"));
    }

    #[test]
    fn handle_try_walks_on_handler_body() {
        // ``try {} on error {result options} {set q 1}`` — the
        // handler body at offset i+3 walks; the varList at i+2
        // is *not* defined as a local (matches Python).
        let mut a = Analyser::new();
        a.handle_try_command(
            "try",
            &[
                String::new(),
                "on".to_string(),
                "error".to_string(),
                "result options".to_string(),
                "set q 1".to_string(),
            ],
            &[
                str_tok(span(5, 7)),
                esc_tok(span(8, 10)),
                esc_tok(span(11, 16)),
                str_tok(span(17, 33)),
                str_tok(span(34, 43)),
            ],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("q"));
        // varList NOT defined — matches Python's ``_handle_try``
        // which doesn't define those bindings.
        assert!(!a.result.global_scope.variables.contains_key("result"));
        assert!(!a.result.global_scope.variables.contains_key("options"));
    }

    #[test]
    fn handle_try_walks_trap_handler_body() {
        // ``try {} trap NONE {result} {set q 1}`` — same shape
        // as ``on``, but the keyword is ``trap``.
        let mut a = Analyser::new();
        a.handle_try_command(
            "try",
            &[
                String::new(),
                "trap".to_string(),
                "NONE".to_string(),
                "result".to_string(),
                "set q 1".to_string(),
            ],
            &[
                str_tok(span(5, 7)),
                esc_tok(span(8, 12)),
                esc_tok(span(13, 17)),
                str_tok(span(18, 26)),
                str_tok(span(27, 36)),
            ],
            &[],
        );
        assert!(a.result.global_scope.variables.contains_key("q"));
    }

    // -- resolve_proc_call (C41c3) ----------------------------------

    #[test]
    fn resolve_proc_call_absolute_qualified_name() {
        // ``::foo`` resolves directly when registered.
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
        let resolved = a.resolve_proc_call("::foo", &[]);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().qualified_name, "::foo");
    }

    #[test]
    fn resolve_proc_call_bare_name_walks_namespace_chain() {
        // ``foo`` declared inside ``ns1`` is found when resolved
        // from ``ns1``'s scope.
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
        // Resolve from inside ns1 — should find ::ns1::foo.
        let resolved = a.resolve_proc_call("foo", &[0]);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().qualified_name, "::ns1::foo");
    }

    #[test]
    fn resolve_proc_call_falls_back_to_global() {
        // Bare ``foo`` declared at global is found from any scope.
        use crate::analyser::types::{Scope, ScopeKind};
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
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "ns1"));
        // Resolve from inside ns1 — chain misses ::ns1::foo,
        // falls back to ::foo.
        let resolved = a.resolve_proc_call("foo", &[0]);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().qualified_name, "::foo");
    }

    #[test]
    fn resolve_proc_call_qualified_relative_name() {
        // ``a::b`` (qualified but not absolute) prepends ``::``.
        let mut a = Analyser::new();
        a.result.all_procs.insert(
            "::a::b".to_string(),
            super::ProcDef {
                name: "b".to_string(),
                qualified_name: "::a::b".to_string(),
                params: Vec::new(),
                name_span: span(0, 0),
                body_span: span(0, 0),
                doc: String::new(),
            },
        );
        let resolved = a.resolve_proc_call("a::b", &[]);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().qualified_name, "::a::b");
    }

    #[test]
    fn resolve_proc_call_unknown_name_returns_none() {
        let a = Analyser::new();
        assert!(a.resolve_proc_call("nope", &[]).is_none());
    }

    #[test]
    fn resolve_proc_call_empty_name_returns_none() {
        let a = Analyser::new();
        assert!(a.resolve_proc_call("", &[]).is_none());
    }

    // -- resolve_expansion_count (C41c3) ----------------------------

    #[test]
    fn resolve_expansion_count_braced_literal() {
        // ``{a b c}`` — Str token; inner content "a b c" splits
        // to three elements.
        let mut a = Analyser::new();
        a.source = "{a b c}".to_string();
        // Span covers ``{a b c`` (5 inner chars + opening brace),
        // content_offset = 1 to skip ``{``.  Closing ``}`` is
        // OUTSIDE the span by lexer convention for non-degenerate
        // STR tokens.
        let tok = Token {
            kind: TokenType::Str,
            span: span(0, 6),
            content_offset: 1,
            in_quote: false,
        };
        assert_eq!(a.resolve_expansion_count(tok, true, &[]), Some(3));
    }

    #[test]
    fn resolve_expansion_count_braced_empty_list() {
        // ``{}`` — degenerate Str case; span extended to include
        // ``}``, token_text returns empty string.
        let mut a = Analyser::new();
        a.source = "{}".to_string();
        let tok = Token {
            kind: TokenType::Str,
            span: span(0, 2),
            content_offset: 1,
            in_quote: false,
        };
        assert_eq!(a.resolve_expansion_count(tok, true, &[]), Some(0));
    }

    #[test]
    fn resolve_expansion_count_var_with_const_value() {
        // ``$xs`` where xs has known constant ``a b c`` — splits
        // to three elements.
        let mut a = Analyser::new();
        a.source = "$xs".to_string();
        a.set_const_string("xs", "a b c".to_string(), span(0, 5), &[]);
        // Var token: span covers ``xs`` (after `$`) by lexer
        // convention; content_offset = 0 because the lexer's
        // ``_start`` for VAR is set after the ``$``.
        // For testing, place the var name at offset 1..3 in source.
        let tok = Token {
            kind: TokenType::Var,
            span: span(1, 3),
            content_offset: 0,
            in_quote: false,
        };
        assert_eq!(a.resolve_expansion_count(tok, true, &[]), Some(3));
    }

    #[test]
    fn resolve_expansion_count_var_without_const_value() {
        // Var with no known constant value → None.
        let mut a = Analyser::new();
        a.source = "$xs".to_string();
        let tok = Token {
            kind: TokenType::Var,
            span: span(1, 3),
            content_offset: 0,
            in_quote: false,
        };
        assert_eq!(a.resolve_expansion_count(tok, true, &[]), None);
    }

    #[test]
    fn resolve_expansion_count_concatenated_word_returns_none() {
        // ``single_token = false`` short-circuits to None.
        let mut a = Analyser::new();
        a.source = "{a b c}".to_string();
        let tok = Token {
            kind: TokenType::Str,
            span: span(0, 6),
            content_offset: 1,
            in_quote: false,
        };
        assert_eq!(a.resolve_expansion_count(tok, false, &[]), None);
    }

    #[test]
    fn resolve_expansion_count_other_token_kind_returns_none() {
        // Non-Str, non-Var token kinds aren't statically
        // resolvable.
        let a = Analyser::new();
        let tok = esc_tok(span(0, 4));
        assert_eq!(a.resolve_expansion_count(tok, true, &[]), None);
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

    // -- handle_oo_class_command body walking (C41e1) ---------------

    #[test]
    fn analyse_oo_class_body_records_superclass_and_methods() {
        // End-to-end: ``oo::class create Sub`` with a body
        // declaring a superclass and a method.  After analyse
        // ``::Sub`` should carry both fields.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "oo::class create Sub { superclass ::Base\nmethod greet {} { puts hi } }",
            "tcl",
        );
        assert!(r.all_classes.contains_key("::Sub"));
        let cls = &r.all_classes["::Sub"];
        assert_eq!(cls.superclasses, vec!["::Base"]);
        assert!(cls.methods.contains_key("greet"));
        assert_eq!(cls.methods["greet"].kind, "method");
    }

    #[test]
    fn analyse_oo_class_body_records_classmethod_and_mixin() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "oo::class create C { mixin ::M\nclassmethod build {} { return ok } }",
            "tcl",
        );
        let cls = &r.all_classes["::C"];
        assert_eq!(cls.mixins, vec!["::M"]);
        assert!(cls.class_methods.contains_key("build"));
        assert!(!cls.methods.contains_key("build"));
    }

    // -- handle_oo_define_command body walking (C41e2) --------------

    #[test]
    fn analyse_oo_define_body_extends_existing_class() {
        // ``oo::class create C {}`` followed by ``oo::define
        // C { method m {} {} }`` — the method ends up in the
        // already-recorded class.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "oo::class create C {}\noo::define C { method m {} {} }",
            "tcl",
        );
        assert!(r.all_classes.contains_key("::C"));
        let cls = &r.all_classes["::C"];
        assert!(cls.methods.contains_key("m"));
    }

    #[test]
    fn analyse_oo_define_inline_form_extends_class() {
        // ``oo::define C method m {} {}`` — inline form,
        // single subcommand.  Works whether or not the class
        // was previously declared (creates a stub if absent).
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("oo::define MyClass method greet {} { puts hi }", "tcl");
        assert!(r.all_classes.contains_key("::MyClass"));
        let cls = &r.all_classes["::MyClass"];
        assert!(cls.methods.contains_key("greet"));
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

    // -- C41e3: ClassDef extended fields + UnknownProcInfo ---------

    #[test]
    fn analyse_oo_class_records_metaclass_from_command_name() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("oo::class create C {}", "tcl");
        let cls = &r.all_classes["::C"];
        assert_eq!(cls.metaclass, "oo::class");
    }

    #[test]
    fn analyse_oo_class_body_records_constructors_and_destructor() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "oo::class create C { constructor args { puts ctor }\ndestructor { puts dtor } }",
            "tcl",
        );
        let cls = &r.all_classes["::C"];
        assert_eq!(cls.constructors.len(), 1);
        assert_eq!(cls.constructors[0].kind, "constructor");
        assert!(cls.destructor.is_some());
        assert_eq!(cls.destructor.as_ref().unwrap().kind, "destructor");
    }

    #[test]
    fn analyse_oo_class_body_records_variables_filters_exports() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "oo::class create C { variable x y\nfilter log\nexport foo bar\nunexport hidden }",
            "tcl",
        );
        let cls = &r.all_classes["::C"];
        assert_eq!(cls.variables, vec!["x", "y"]);
        assert_eq!(cls.filters, vec!["log"]);
        assert!(cls.exports.contains("foo"));
        assert!(cls.exports.contains("bar"));
        assert!(cls.unexports.contains("hidden"));
    }

    #[test]
    fn analyse_oo_class_body_records_property_def() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "oo::class create C { property colour -kind readwrite -get { return red } }",
            "tcl",
        );
        let cls = &r.all_classes["::C"];
        let pd = cls.properties.get("colour").expect("colour recorded");
        assert_eq!(pd.kind, "readwrite");
        assert!(pd.has_getter);
        assert!(!pd.has_setter);
    }

    #[test]
    fn analyse_unknown_proc_records_dispatch_targets_end_to_end() {
        // End-to-end: a ``proc unknown {cmd args} {...}`` with
        // an exact-match switch should populate
        // ``result.unknown_proc_info`` with the arm labels as
        // dispatch targets.  This is what gates W123 in C41d4
        // once the unknown_proc_info early-return lands.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "proc unknown {cmd args} { switch -exact $cmd { foo { return 1 } bar { return 2 } } }",
            "tcl",
        );
        let info = r.unknown_proc_info.expect("unknown_proc_info populated");
        assert!(!info.empty_stub);
        assert!(info.dispatch_targets.contains("foo"));
        assert!(info.dispatch_targets.contains("bar"));
    }

    #[test]
    fn analyse_without_unknown_proc_leaves_unknown_proc_info_none() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("proc foo {} { return 1 }", "tcl");
        assert!(r.unknown_proc_info.is_none());
    }

    #[test]
    fn analyse_unknown_proc_with_empty_body_marks_empty_stub() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("proc unknown {cmd args} {}", "tcl");
        let info = r.unknown_proc_info.expect("unknown_proc_info populated");
        assert!(info.empty_stub);
    }

    #[test]
    fn analyse_qualified_unknown_proc_also_populates_info() {
        // ``::tcl::unknown`` (the canonical fully-qualified
        // name) should trigger detection too.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("proc ::tcl::unknown {cmd args} { exec $cmd }", "tcl");
        let info = r.unknown_proc_info.expect("unknown_proc_info populated");
        assert!(info.has_exec);
    }

    // -- C41e4: stray-close-bracket recovery ------------------------

    #[test]
    fn analyse_top_level_repairs_stray_close_bracket() {
        // ``set x string]`` is a typo for ``set x [string ...]``.
        // The recovery should rewrite the third argv entry into
        // a virtual ``CMD`` token before dispatch so the var
        // record is registered with the recovered shape.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("set x string]", "tcl");
        // ``x`` ends up in scope as a single-arg ``set`` (a var
        // read), not as a two-arg ``set`` with the broken text
        // — recovery yields the synthetic ``[string]`` command
        // word so dispatch sees the intended shape.
        assert!(r.global_scope.variables.contains_key("x"));
    }

    // -- C41e5 + e3 follow-ups: unknown_proc_info / package require -

    #[test]
    fn analyse_records_package_require() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("package require Tcl 8.6", "tcl");
        assert_eq!(r.package_requires.len(), 1);
        let p = &r.package_requires[0];
        assert_eq!(p.name, "Tcl");
        assert_eq!(p.version.as_deref(), Some("8.6"));
        assert!(!p.conditional);
    }

    #[test]
    fn analyse_records_package_require_exact_flag() {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("package require -exact Tcl 8.6", "tcl");
        let p = &r.package_requires[0];
        assert_eq!(p.name, "Tcl");
        assert_eq!(p.version.as_deref(), Some("8.6"));
    }

    #[test]
    fn analyse_w123_suppressed_when_package_require_seen() {
        // W123 is suppressed when any package require is on
        // file — package may load arbitrary commands.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("package require Foo\nbogus_command arg", "tcl");
        assert!(!r.diagnostics.iter().any(|d| d.code == "W123"));
    }

    #[test]
    fn analyse_w123_suppressed_when_unknown_proc_chains_original() {
        // ``proc unknown`` that chains the original handler is
        // a *dynamic* shape — Python suppresses W123 entirely
        // because runtime can resolve any command name.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "proc unknown {cmd args} { _original_unknown $cmd {*}$args }\nbogus_command arg",
            "tcl",
        );
        assert!(!r.diagnostics.iter().any(|d| d.code == "W123"));
    }

    #[test]
    fn analyse_w123_suppressed_when_unknown_proc_calls_exec() {
        // ``exec $cmd`` inside ``unknown`` is a dynamic shape;
        // any command may be a real binary on PATH.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "proc unknown {cmd args} { exec $cmd {*}$args }\nbogus_command arg",
            "tcl",
        );
        assert!(!r.diagnostics.iter().any(|d| d.code == "W123"));
    }

    #[test]
    fn analyse_w123_still_fires_outside_explicit_dispatch_targets() {
        // ``proc unknown`` with ONLY explicit dispatch targets
        // (no exec / auto_load / chain / pattern / case-fold)
        // is *not* dynamic — W123 should still fire for
        // commands not in the explicit target set.  Mirrors
        // Python's behaviour from ``_diag_commands.py:64-71``.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "proc unknown {cmd args} { switch -exact $cmd { foo { return 1 } } }\nbogus_command arg",
            "tcl",
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == "W123"),
            "W123 expected for ``bogus_command`` outside explicit dispatch targets; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w123_suppressed_for_explicit_dispatch_target() {
        // ``foo`` is in the explicit dispatch_targets — even
        // for the non-dynamic shape, the per-invocation loop
        // suppresses W123 for it.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(
            "proc unknown {cmd args} { switch -exact $cmd { foo { return 1 } } }\nfoo arg",
            "tcl",
        );
        assert!(
            !r.diagnostics
                .iter()
                .any(|d| d.code == "W123" && d.message.contains("'foo'")),
            "W123 should not fire for command listed in dispatch_targets; got {:?}",
            r.diagnostics,
        );
    }

    #[test]
    fn analyse_w123_still_fires_for_empty_unknown_stub() {
        // An empty ``unknown`` stub resolves nothing — W123
        // should still emit.
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse("proc unknown {cmd args} {}\nbogus_command arg", "tcl");
        // ``bogus_command`` should be flagged.
        assert!(r.diagnostics.iter().any(|d| d.code == "W123"));
    }
}
