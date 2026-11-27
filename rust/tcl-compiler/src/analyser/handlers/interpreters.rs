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

//! Child interpreters: `interp create`, `delete`, `hide`, `expose` and the scripts `interp eval` runs in a child.

use super::super::state::Analyser;
use tcl_lexer::Token;

/// The child an invocation's transitions create: its path (`None` for an
/// auto-named child) and whether it starts safe — `None` when the call states
/// no creation (another command, or a shape Tcl rejects). A child whose
/// safety a computed option leaves unknown is not recorded as safe.
pub(super) fn created_interpreter(
    transitions: &tcl_registry::StateTransitions,
) -> Option<(Option<tcl_registry::TransitionSubject>, bool)> {
    transitions
        .facts()
        .iter()
        .find_map(|fact| match &fact.transition {
            tcl_registry::StateTransition::Interpreter(
                tcl_registry::InterpreterTransition::Create {
                    interpreter,
                    safety,
                },
            ) => Some((
                interpreter.clone(),
                *safety == tcl_registry::ChildInterpreterSafety::Safe,
            )),
            _ => None,
        })
}

impl Analyser {
    pub(super) fn bind_created_interpreter_value(
        &mut self,
        path: Option<&tcl_registry::TransitionSubject>,
        safe: bool,
        name: &str,
        value_tok: Token,
        scope_path: &[usize],
    ) {
        // The substitution's registry transition, not its spelling, names the
        // created interpreter. Keep the variable binding in the scope chain.
        use tcl_registry::TransitionSubject;
        self.clear_const_string(name, scope_path);
        let written = match path {
            Some(
                TransitionSubject::Literal(value) | TransitionSubject::LocatedLiteral { value, .. },
            ) => Some(Some(value.as_str())),
            Some(TransitionSubject::LocatedNativeBytes { value, .. }) => {
                Some(std::str::from_utf8(value).ok())
            }
            Some(TransitionSubject::Unknown { .. }) => Some(None),
            None => None,
        };
        match written {
            Some(None) => {
                self.dynamic_interp_ops = true;
                self.clear_interp_var_binding(name, scope_path);
            }
            written => {
                let key = match written {
                    Some(Some(p)) => self.qualified_interp_key(p),
                    _ => self.mint_synthetic_offset_name("@autoname@", value_tok.span.start()),
                };
                self.interpreters.insert(
                    key.clone(),
                    super::super::state::InterpState {
                        safe,
                        ..Default::default()
                    },
                );
                self.set_original_logical_interp_var_binding(name, key, value_tok, scope_path);
            }
        }
    }

    /// Handle `interp eval PATH SCRIPT`: the script runs in a **child**
    /// interpreter — a separate command / variable space — so its `proc` /
    /// `oo::class` / variable definitions and calls must not merge into the
    /// parent namespace (a parent `rename foo` must not rewrite a child `proc
    /// foo`; [`tcl_vm::interp`] isolates the child at run time).
    ///
    /// Only the single-script form (`interp eval child { … }`) is isolated: an
    /// **empty** path (`interp eval {} script`) targets the *current*
    /// interpreter — its definitions belong here — and a multi-word /
    /// non-literal script cannot be statically re-assembled, so both fall back
    /// to the generic body recursion by returning `false`.  Otherwise an
    /// isolated child scope is opened, named for the interpreter path so the
    /// child's definitions home under it (`::<path>::foo`) and the child's own
    /// calls still resolve within the block; a dynamic path (`$i`) can't
    /// collide with a real namespace, so it stays conservatively isolated too.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::InterpEval`]
    /// (stamped on `interp`'s `eval` subcommand); `args[0]` is the subcommand
    /// word.
    pub fn handle_interp_eval_command(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        self.handle_interp_eval_command_original(args, arg_tokens, scope_path, None)
    }

    pub(in super::super) fn handle_interp_eval_command_original(
        &mut self,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        head: Option<Token>,
    ) -> bool {
        // `interp eval PATH arg ?arg ...?` — the subcommand word, the path,
        // and one or more script words (C concatenates them).
        if args.len() < 3 {
            return false;
        }
        // An empty path runs in the current interpreter — not isolated.
        if args[1].is_empty() {
            return false;
        }
        // The interpreter path is a *list* relative to the current
        // interpreter — inside a child's eval body it is relative to that
        // child — so qualify against the walk's interpreter-path stack. A
        // dynamic path also resolves through a tracked `set VAR [interp
        // create ...]` binding — that key is already
        // fully qualified at bind time, so it's used as-is, not
        // requalified.
        let literal_path = !crate::naming::is_dynamic_word(&args[1]);
        let resolved_dynamic = (!literal_path)
            .then(|| self.resolve_dynamic_interp_path(arg_tokens.get(1).copied()))
            .flatten();
        let original_key =
            head.and_then(|head| self.original_interp_eval_state_key(head, arg_tokens));
        // An unresolved computed path keeps a domain of its own site: its
        // written text names no interpreter, and two procedures that both
        // spell `$s` hold unrelated ones.
        let key = match (original_key, resolved_dynamic.clone(), arg_tokens.get(1)) {
            (Some(key), _, _) | (None, Some(key), _) => key,
            (None, None, Some(path)) if !literal_path => {
                let site = self.mint_synthetic_offset_name("@dynns@", path.span.start());
                self.qualified_interp_key(&site)
            }
            (None, None, _) => self.qualified_interp_key(&args[1]),
        };
        if literal_path || resolved_dynamic.is_some() {
            // Interpreter existence: evaluating into a
            // known-but-never-created child raises `could not find
            // interpreter` at run time.  Abstains when any interp operation
            // in the file used a dynamic, *unresolvable* path (existence
            // then unknowable).
            if !self.interpreters.contains_key(&key)
                && !self.dynamic_interp_ops
                && let Some(tok) = arg_tokens.get(1)
            {
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        tcl_core_types::DiagCode::W140,
                        tok.span,
                        format!(
                            "interpreter '{}' is never created in this file — \
                         `interp eval` will raise `could not find interpreter`",
                            args[1]
                        ),
                        super::super::types::Severity::Warning,
                    ));
            }
        }
        // Multiple script words concatenate at run time (`Tcl_ConcatObj`),
        // so commands can span word boundaries — no per-word walk is sound.
        // Consume the command *without* walking, keeping the parent domain
        // clean (a fall-through would analyse the words in the parent
        // scope); W312 separately flags the shape.
        if args.len() > 3 {
            return true;
        }
        let Some(body_tok) = arg_tokens.get(2).copied() else {
            return true;
        };
        // A path we could not resolve still gets its own domain (two
        // `interp eval $a {…}` / `interp eval $b {…}` bodies must not
        // merge), but it is flagged unresolved so per-interpreter state
        // widens rather than asserting this is a *different* interpreter
        // from every other — `$path` could hold any name, including one
        // already used literally elsewhere in the file.
        let resolved = literal_path || resolved_dynamic.is_some();
        let visibility = head.and_then(|head| {
            self.original_interp_visibility_body(&key, body_tok, head, arg_tokens)
        });
        self.isolate_interp_eval_body(&key, resolved, &args[2], body_tok, scope_path, visibility);
        true
    }

    /// Isolate `body_text` — already known to belong to interpreter `key` —
    /// into a synthetic `@interp@<key>` child scope, so its `proc` /
    /// `oo::class` / variable definitions and calls don't merge into the
    /// parent namespace. Shared core of [`Self::handle_interp_eval_command`]
    /// (literal `interp eval PATH { … }`) and
    /// [`Self::handle_interp_handle_eval_command`] (`NAME eval { … }` via the
    /// interpreter's own object command) — both recognise the *same*
    /// isolated-body shape, just via different call-site spellings, so both
    /// must isolate it identically rather than maintaining two copies of
    /// this logic that could drift apart.
    ///
    /// `path_resolved` says whether `key` names an interpreter the walk could
    /// identify statically (a literal path, or a dynamic one resolved through
    /// a tracked `set VAR [interp create …]` binding).  It is recorded on the
    /// pushed [`InterpFrame`](super::super::state::InterpFrame) so consumers of the
    /// domain identity — analyser state that models *per-interpreter runtime
    /// state*, such as the Tk widget/geometry hierarchy — can
    /// widen conservatively for a body whose interpreter is unknowable
    /// instead of treating it as provably distinct from every other domain.
    ///
    /// Returns `false` (never isolated) only when the scope-tree path has
    /// gone stale — shouldn't happen during a healthy walk.
    fn isolate_interp_eval_body(
        &mut self,
        key: &str,
        path_resolved: bool,
        body_text: &str,
        body_tok: Token,
        scope_path: &[usize],
        visibility: Option<super::super::SourceInterpreterVisibilitySnapshot>,
    ) -> bool {
        let outer = scope_path.to_vec();
        let domain = self.interp_domain_name(key);
        let child_scope_idx = {
            // The child's global namespace is its own domain, not a parent
            // namespace — home the scope under a synthetic
            // `@interp@<path>` name (unrepresentable in Tcl, mirroring
            // `@objdefine@`) so a real parent namespace of the same name
            // can never collide.  Repeated evals into the same live
            // interpreter share one domain name (their definitions
            // accumulate, as in C); the name carries the path's deletion
            // *epoch*, so a deleted-and-recreated interpreter is a fresh
            // domain that never merges with its predecessor's definitions.
            let mut child = super::super::types::Scope::new(
                super::super::types::ScopeKind::Namespace,
                domain.clone(),
            );
            child.body_span = Some(body_tok.span);
            child.interpreter_source_body = super::super::types::InterpreterSourceBody::Independent;
            child.original_interpreter_source_domain = visibility
                .as_ref()
                .map(|snapshot| snapshot.reporting_domain(domain.clone()));
            child.naming_scope = visibility.as_ref().map(|_| {
                tcl_names::namespace::SignatureNamespaceScope::C(
                    tcl_core_types::ByteNamespacePath::root(),
                )
            });
            let Some(parent) =
                super::super::scope::scope_at_mut(&mut self.result.global_scope, &outer)
            else {
                return false;
            };
            parent.children.push(child);
            parent.children.len() - 1
        };
        let mut child_path = outer;
        child_path.push(child_scope_idx);

        // Each child source has an independent context, including a child
        // whose original declaration cannot be retained. The enclosing
        // child's snapshot must not leak into that unknown body.
        let outer_visibility = std::mem::take(&mut self.safe_interp_stack);
        if let Some(context) = visibility {
            self.safe_interp_stack.push(context);
        }
        // `interp` operations inside the body name paths relative to *this*
        // child — push its path so they qualify correctly.  The frame also
        // carries the body's domain identity for per-interpreter state; an
        // unresolvable path anywhere in the enclosing chain makes the whole
        // frame unresolved, since a body nested inside an unknowable
        // interpreter is itself in an unknowable one.
        let enclosing_resolved = self.interp_path_stack.last().is_none_or(|f| f.resolved);
        self.interp_path_stack
            .push(super::super::state::InterpFrame {
                key: key.to_string(),
                domain,
                resolved: path_resolved && enclosing_resolved,
            });
        self.analyse_body(body_text, body_tok, &child_path);
        self.interp_path_stack.pop();
        self.safe_interp_stack = outer_visibility;
        true
    }

    /// Handle `NAME eval SCRIPT` — a far more common real-world spelling of
    /// `interp eval NAME SCRIPT` than the `interp eval` form itself: `interp
    /// create` binds the child interpreter's *own* object command to its
    /// create-time name (`interp create sandbox` makes `sandbox` itself
    /// callable; `sandbox eval { … }` dispatches exactly like `interp eval
    /// sandbox { … }`). Without this, the body is neither isolated nor
    /// walked at all: no symbols are produced for procs defined inside it,
    /// and hover/go-to-definition on a call inside falls back to a
    /// scope-blind, file-wide "any proc anywhere with this bare name" match
    /// — which can resolve to a same-named proc in a completely unrelated
    /// interpreter.
    ///
    /// Recognised purely from analysis state — `cmd_name` is looked up
    /// against [`Self::interpreters`] (built by
    /// [`Self::handle_interp_create_command`]), never matched against a
    /// hardcoded name — so an ordinary proc that happens to be named `eval`
    /// as its first argument (`foo eval bar`, `foo` an unrelated command) is
    /// untouched. Only the single-script form is isolated, mirroring
    /// [`Self::handle_interp_eval_command`]; every other shape (no `eval`
    /// first word, wrong arity, an untracked head) falls through to the
    /// generic per-command dispatch by returning `false`.
    ///
    /// Handles the `$handle eval { … }` spelling too: `cmd_name` is the
    /// literal, unsubstituted head text, so a `$`-prefixed handle only
    /// resolves through a tracked `set VAR [interp create ...]` binding
    /// ([`Self::resolve_dynamic_interp_path`]) — a handle sourced any
    /// other way (a proc parameter, a value read from elsewhere) still
    /// falls through untouched, the same conservative fallback used for
    /// an untracked head.
    ///
    /// Dispatched from [`Self::dispatch_analyser_hook`]'s hookless fallback
    /// chain — `cmd_name` never matches a registry command (it's a
    /// user-chosen interpreter name), so this can't be reached via a
    /// registry hook the way the literal `interp eval` form is.
    pub fn handle_interp_handle_eval_command(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
    ) -> bool {
        self.handle_interp_handle_eval_command_original(
            cmd_name, args, arg_tokens, scope_path, None,
        )
    }

    pub(in super::super) fn handle_interp_handle_eval_command_original(
        &mut self,
        cmd_name: &str,
        args: &[String],
        arg_tokens: &[Token],
        scope_path: &[usize],
        head: Option<Token>,
    ) -> bool {
        // registry-axis-ok: subcommands — a namespace eval wrapper's subcommand; until the resolved invocation's selected subcommand is read
        if args.len() != 2 || args[0] != "eval" {
            return false;
        }
        let Some(body_tok) = arg_tokens.get(1).copied() else {
            return false;
        };
        if let Some((key, visibility)) = head.and_then(|head| {
            self.original_interp_handle_visibility_body(body_tok, head, arg_tokens)
        }) {
            return self.isolate_interp_eval_body(
                &key,
                true,
                &args[1],
                body_tok,
                scope_path,
                Some(visibility),
            );
        }
        let key = if crate::naming::is_dynamic_word(cmd_name) {
            let Some(resolved) = self.resolve_dynamic_interp_path(head) else {
                return false;
            };
            resolved
        } else {
            self.qualified_interp_key(cmd_name)
        };
        if !self.interpreters.contains_key(&key) {
            return false;
        }
        // Compatibility reporting retains the tracked interpreter path, but
        // no genuine source receipt or visibility advice follows from it.
        self.isolate_interp_eval_body(&key, true, &args[1], body_tok, scope_path, None)
    }

    /// Qualify a literal `interp` path operand against the enclosing
    /// `interp eval` bodies on the walk stack: paths are relative to the
    /// *current* interpreter, so `interp create t` inside
    /// `interp eval s {…}` names `s t`.  A dynamic operand is returned
    /// key-normalised only (its stack context cannot make it literal).
    pub(in super::super) fn qualified_interp_key(&self, path: &str) -> String {
        // The path is a Tcl list naming a descent through children (`{s t}`
        // is child `t` of child `s`); its canonical list form is the key.
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&self.file_lexer_config());
        let local = rules.split_list(path).map_or_else(
            |_| path.to_owned(),
            |elements| tcl_syntax::list::join_list(elements.iter()),
        );
        match self.interp_path_stack.last() {
            Some(enclosing) if !local.is_empty() => format!("{} {local}", enclosing.key),
            _ => local,
        }
    }

    /// The `::`-rooted prefix that qualifies a name into the interpreter
    /// domain the walk is currently inside: empty at the top level (the main
    /// interpreter), `::@interp@<path>[#<epoch>]` inside a child's eval body.
    ///
    /// Concatenate it with an already-`::`-rooted name (`::foo` →
    /// `::@interp@c::foo`) or use it as the namespace half of a join. It is
    /// what command-table facts (`interp alias`, alias deletion) written with
    /// an *empty* interpreter path must be keyed by: an empty path means "the
    /// interpreter this command runs in", which is the child inside a
    /// `child eval { … }` body, not the main interpreter.
    pub(in super::super) fn current_interp_domain_prefix(&self) -> String {
        self.interp_path_stack
            .last()
            .map_or_else(String::new, |frame| {
                crate::naming::root_unrooted_key(&frame.domain)
            })
    }

    /// The synthetic namespace name for an interpreter domain: the
    /// current epoch of `key` is folded in so a deleted-and-recreated
    /// interpreter never shares its predecessor's definitions.
    fn interp_domain_name(&self, key: &str) -> String {
        match self.interp_epochs.get(key) {
            Some(epoch) if *epoch > 0 => format!("@interp@{key}#{epoch}"),
            _ => format!("@interp@{key}"),
        }
    }

    /// Current original scalar syntax selects independently recorded interpreter-path advice.
    pub(super) fn resolve_dynamic_interp_path(&self, token: Option<Token>) -> Option<String> {
        self.lookup_logical_interp_path(token?)
    }

    /// Resolve an `interp alias` path operand to the domain prefix its
    /// alias/target command name qualifies under: the
    /// plain-current-interpreter sentinel (`""`/`"{}"`) is the empty
    /// prefix; a literal word resolves via [`Self::qualified_interp_key`]
    /// and then [`Self::interp_domain_name`]; a dynamic word resolves
    /// ONLY through [`Self::resolve_dynamic_interp_path`] — anything else
    /// (an untracked dynamic word) returns `None`, aborting the whole
    /// cross-domain alias (unchanged conservative behaviour).
    pub(super) fn resolve_alias_domain_prefix(
        &self,
        path: &str,
        token: Option<Token>,
    ) -> Option<String> {
        if matches!(path, "" | "{}") {
            // The empty path is *this* interpreter — which is the child when
            // the command is written inside a child's eval body, not always
            // the main one.
            return Some(self.current_interp_domain_prefix());
        }
        let key = if crate::naming::is_dynamic_word(path) {
            self.resolve_dynamic_interp_path(token)?
        } else {
            self.qualified_interp_key(path)
        };
        Some(crate::naming::root_unrooted_key(
            &self.interp_domain_name(&key),
        ))
    }

    /// Handle `interp create ?-safe? ?--? ?path?` — record the child
    /// interpreter's existence and safe state in the interpreter-domain
    /// map, read off the invocation's `InterpreterTransition::Create`: the
    /// registry resolver scans the options as Tcl does (the scan continues
    /// past the path, `-safe` and `--` match by prefix, and a shape Tcl
    /// rejects states no creation). A computed path makes interpreter
    /// existence unknowable file-wide; a missing path auto-generates a name
    /// this file cannot reference literally, so nothing is recorded.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::InterpCreate`],
    /// with the transitions the call states over its source words.
    /// Returns the literal path the call created, for visibility capture.
    pub fn handle_interp_create_command(
        &mut self,
        transitions: &tcl_registry::StateTransitions,
    ) -> Option<String> {
        let (Some(subject), safe) = created_interpreter(transitions)? else {
            return None;
        };
        {
            let path = subject.literal().or_else(|| {
                subject
                    .native_bytes()
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
            });
            let Some(path) = path else {
                self.dynamic_interp_ops = true;
                return None;
            };
            let state = super::super::state::InterpState {
                safe,
                ..Default::default()
            };
            let key = self.qualified_interp_key(path);
            self.interpreters.insert(key, state);
            Some(path.to_owned())
        }
    }

    /// Handle `interp delete ?path ...?` — remove the recorded state and
    /// bump the path's **epoch**, so a later re-creation is a fresh
    /// domain.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::InterpDelete`].
    pub fn handle_interp_delete_command(&mut self, args: &[String]) {
        for word in &args[1..] {
            if crate::naming::is_dynamic_word(word) {
                self.dynamic_interp_ops = true;
                continue;
            }
            let key = self.qualified_interp_key(word);
            if self.interpreters.remove(&key).is_some() {
                *self.interp_epochs.entry(key).or_insert(0) += 1;
            }
        }
    }

    /// Compatibility entry without original operands. It withdraws source
    /// visibility advice; the complete invocation entry retains hidden tokens
    /// separately from scoped visible slots.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::InterpHide`].
    pub fn handle_interp_hide_command(&mut self, args: &[String]) {
        self.apply_interp_visibility_delta(args, true);
    }

    /// Compatibility entry without original operands. It withdraws source
    /// visibility advice rather than constructing hidden-token/name receipts.
    ///
    /// Dispatched via [`tcl_registry::hooks::AnalyserHookId::InterpExpose`].
    pub fn handle_interp_expose_command(&mut self, args: &[String]) {
        self.apply_interp_visibility_delta(args, false);
    }

    /// Compatibility handlers without original operands cannot issue visibility.
    fn apply_interp_visibility_delta(&mut self, args: &[String], _hide: bool) {
        let Some(path) = args.get(1) else {
            return;
        };
        let key = self.qualified_interp_key(path);
        if let Some(state) = self.interpreters.get_mut(&key) {
            state.tainted = true;
            state.visibility = None;
        }
    }

    pub(in super::super) fn handle_interp_hide_command_original(
        &mut self,
        args: &[String],
        head: Token,
        tokens: &[Token],
    ) {
        if let Some(path) = args.get(1) {
            self.apply_original_interp_visibility_delta(path, head, tokens, true);
        }
    }

    pub(in super::super) fn handle_interp_expose_command_original(
        &mut self,
        args: &[String],
        head: Token,
        tokens: &[Token],
    ) {
        if let Some(path) = args.get(1) {
            self.apply_original_interp_visibility_delta(path, head, tokens, false);
        }
    }
}
