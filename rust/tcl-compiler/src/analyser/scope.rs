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

//! Scope-graph helpers.
//!
//! Pure scope-tree mutation and traversal helpers used by the
//! analyser's command handlers. The scope tree uses a path-based
//! addressing scheme (``Vec<usize>`` indexing into
//! ``result.global_scope.children``) rather than parent
//! back-pointers, so any helper that needs to walk up the chain
//! accepts the path slice and traverses the result tree in place.
//! The cost — at most one descent from root per helper call — is
//! bounded by scope depth, which is shallow in practice.
//!
//! Helpers operate on ``&mut Analyser`` rather than free
//! functions because most need both the scope under cursor
//! (resolved from the path) and access to the analyser's
//! tracking maps (``const_strings``, ``regex_vars``, ``ns_cache``,
//! ``result.regex_patterns``).

use tcl_core_types::DiagCode;
use tcl_lexer::{Span, Token, TokenType};

use crate::naming::{normalise_qualified_name, normalise_var_name, split_array_name};

use super::state::Analyser;
use super::types::{Scope, ScopeKind, VarDef};

/// Ancestor path enumerator: yields the active scope's path and
/// each of its proper ancestors back to the root, longest first.
///
/// `[2, 1, 0]` yields `[2, 1, 0]`, `[2, 1]`, `[2]`, `[]`. Used by
/// every helper that walks up the parent chain.
fn ancestor_paths(start: &[usize]) -> impl Iterator<Item = Vec<usize>> + '_ {
    (0..=start.len()).rev().map(|i| start[..i].to_vec())
}

/// Resolve a scope path to a `&Scope` reference inside `root`.
///
/// Walks `path` index-by-index. Returns the root for an empty
/// path; returns `None` if any index is out of bounds.
fn scope_at<'a>(root: &'a Scope, path: &[usize]) -> Option<&'a Scope> {
    let mut cursor = root;
    for &idx in path {
        cursor = cursor.children.get(idx)?;
    }
    Some(cursor)
}

/// Append a namespace component `part` onto the absolute namespace
/// `ns` (`"::"`-rooted), mirroring the join in
/// [`Analyser::namespace_from_scope_path`]: an absolute `part` rebases,
/// a relative one is appended.
fn join_namespace(ns: &str, part: &str) -> String {
    if part.starts_with("::") {
        normalise_qualified_name(part)
    } else if ns == "::" {
        normalise_qualified_name(&format!("::{part}"))
    } else {
        normalise_qualified_name(&format!("{ns}::{part}"))
    }
}

/// Mutable counterpart to [`scope_at`].
pub(super) fn scope_at_mut<'a>(root: &'a mut Scope, path: &[usize]) -> Option<&'a mut Scope> {
    let mut cursor = root;
    for &idx in path {
        cursor = cursor.children.get_mut(idx)?;
    }
    Some(cursor)
}

/// Advance a command-resolution namespace accumulator across one child
/// scope, per Tcl's command-resolution rule for the scope kinds this
/// analyser models:
///
/// * [`ScopeKind::Namespace`] appends its segment — `ns` accumulates
///   through every enclosing `namespace eval`, however deeply nested.
/// * [`ScopeKind::Proc`] / [`ScopeKind::Method`] **reset** to that
///   definition's own defining namespace (the prefix of its qualified
///   name) rather than accumulating lexically: a proc resolves its body's
///   unqualified calls in its own namespace, not necessarily its lexical
///   nesting (a `proc ::a::b::p {}` declared at the top level still
///   resolves unqualified calls against `::a::b`). **Exception:** a
///   `TclOO` method scope ([`Scope::oo_global_resolution`]) resets to
///   global — the body runs in the object's namespace, and the class's
///   defining namespace is never searched (tclsh-pinned).
/// * [`ScopeKind::Uplevel`] (`uplevel #0`) resets to global — the body
///   runs in the global frame.
/// * [`ScopeKind::Global`] is a no-op.
///
/// Shared by [`Analyser::command_resolution_namespace`] (walks a live
/// `scope_path: &[usize]` during the analyser's own body walk) and
/// [`command_resolution_namespace_at`] (walks a fully-built [`Scope`] tree
/// by byte offset, for post-walk LSP consumers with no `scope_path`) so
/// the two traversal mechanisms can never disagree on the underlying rule.
fn advance_command_resolution_namespace(ns: &str, child: &Scope) -> String {
    match child.kind {
        ScopeKind::Namespace => join_namespace(ns, &child.name),
        // A `TclOO` method body executes with the *object's* namespace current
        // (`::oo::ObjN`, path `::oo::Helpers`) — the class's defining
        // namespace is never searched, so the static approximation is
        // global-only resolution (tclsh 8.6.16 / 9.0.4-pinned; see
        // `Scope::oo_global_resolution`). snit / itcl methods fall through
        // to the defining-namespace rule below.
        ScopeKind::Method if child.oo_global_resolution => "::".to_string(),
        ScopeKind::Proc | ScopeKind::Method => {
            let qualified = if child.name.starts_with("::") {
                normalise_qualified_name(&child.name)
            } else {
                join_namespace(ns, &child.name)
            };
            match qualified.rsplit_once("::") {
                Some((prefix, _)) if !prefix.is_empty() => prefix.to_string(),
                _ => "::".to_string(),
            }
        }
        ScopeKind::Global => ns.to_string(),
        ScopeKind::Uplevel => "::".to_string(),
    }
}

/// Namespace an unqualified command invoked at `byte_offset` resolves
/// against, per Tcl's command-resolution rule.
///
/// The byte-offset-based sibling of [`Analyser::command_resolution_namespace`]
/// (which walks a live `scope_path` during the analyser's own body walk):
/// this variant walks a fully-built [`Scope`] tree by byte position, for LSP
/// providers that only have a finished `AnalysisResult` and a cursor / call-site
/// offset — no `scope_path`. `tcl-lsp-core`'s find-references / rename /
/// call-hierarchy namespace gates all resolve through this one function
/// (via [`advance_command_resolution_namespace`], the shared per-scope-kind
/// rule) so they can't disagree with each other or with the analyser's own
/// resolution.
///
/// Descends the innermost child scope whose `body_span` contains
/// `byte_offset` at each level, exactly like `tcl-lsp-core`'s
/// `scope_chain_at` — a scope with no `body_span` (or one that doesn't
/// contain the offset) simply isn't descended into.
#[must_use]
pub fn command_resolution_namespace_at(root: &Scope, byte_offset: u32) -> String {
    let mut ns = "::".to_string();
    let mut cursor = root;
    loop {
        let next = cursor.children.iter().find(|c| {
            c.body_span
                .is_some_and(|s| s.start() <= byte_offset && byte_offset < s.end())
        });
        let Some(child) = next else { break };
        ns = advance_command_resolution_namespace(&ns, child);
        cursor = child;
    }
    ns
}

impl Analyser {
    /// Resolve the current scope path to a borrow of the active
    /// [`Scope`] inside [`Self::result`]. Convenience wrapper
    /// around [`scope_at`] using ``self.current_scope_path`` as
    /// the path; falls back to the global scope if the path
    /// has gone stale (shouldn't happen during a healthy walk).
    #[must_use]
    pub fn current_scope(&self) -> &Scope {
        scope_at(&self.result.global_scope, &self.current_scope_path)
            .unwrap_or(&self.result.global_scope)
    }

    /// Mutable counterpart to [`Self::current_scope`].
    pub fn current_scope_mut(&mut self) -> &mut Scope {
        let path = self.current_scope_path.clone();
        scope_at_mut(&mut self.result.global_scope, &path)
            .expect("current_scope_path must be valid")
    }

    /// Walk every `Var` token in a segmented command and call
    /// [`Self::record_var_read`] for each.  This is how the
    /// analyser tracks `$x` substitutions in arg positions
    /// (`puts $x`, `string length $name`, etc.) — without it,
    /// `VarDef.references` would only carry the explicit
    /// single-arg `set x` read sites that `handle_set_command`
    /// records.
    ///
    /// The token-text helper uses [`tcl_lexer::SourceMap`] to
    /// recover each `$name` token's textual content from the
    /// source bytes — `Var` tokens carry only their span, not
    /// the inner name.  The `$` prefix is stripped; `${name}`
    /// braced forms are decoded down to `name`.
    pub fn record_arg_var_reads(
        &mut self,
        cmd: &crate::segmenter::SegmentedCommand,
        scope_path: &[usize],
    ) {
        use tcl_lexer::TokenType;
        // Collect (name, span) tuples in a first pass so the
        // source borrow releases before we mutate `self` via
        // `record_var_read`.  Token spans that exceed the
        // source bounds (which happens in test harnesses that
        // pass synthetic tokens against an empty source) are
        // silently skipped — they aren't reading anything
        // meaningful.
        let head_span = cmd.argv.first().map(|t| t.span);
        let source_len = u32::try_from(self.source.len()).unwrap_or(u32::MAX);
        let mut reads: Vec<(String, tcl_lexer::Span)> = Vec::new();
        for tok in &cmd.all_tokens {
            if tok.kind != TokenType::Var {
                continue;
            }
            if head_span.is_some_and(|hs| tok.span == hs) {
                continue;
            }
            if tok.span.end() > source_len {
                continue;
            }
            let Some(name) = var_name_from_span(&self.source, tok.span) else {
                continue;
            };
            reads.push((name.to_string(), tok.span));
        }
        for (name, span) in reads {
            self.record_var_read(&name, span, scope_path);
        }

        // `$var` reads also occur inside command substitutions
        // (`[…]`) and braced `expr` / condition arguments, which the
        // main body walk treats as opaque tokens.  Collect those
        // here too so `VarDef.references` is complete for the
        // references / document-highlight providers and the
        // minifier's rename pass.  `record_var_read` no-ops on names
        // that aren't in the current scope, so liberal collection is
        // safe.
        if let Some(registry) = self.registry {
            let mut extra: Vec<(String, tcl_lexer::Span)> = Vec::new();
            let cmd_name = cmd.texts.first().map_or("", String::as_str);
            let post: Vec<&str> = cmd.texts.iter().skip(1).map(String::as_str).collect();
            let expr_idx: std::collections::HashSet<usize> = registry
                .arg_indices_for_role(cmd_name, &post, tcl_registry::ArgRole::Expr)
                .into_iter()
                .collect();
            // Top-level command-substitution tokens.
            for tok in &cmd.all_tokens {
                if tok.kind == tcl_lexer::TokenType::Cmd {
                    collect_cmd_subst_reads(&self.source, *tok, registry, &mut extra);
                }
            }
            // Braced expr arguments.
            for (i, arg) in cmd.argv.iter().enumerate().skip(1) {
                if arg.kind == tcl_lexer::TokenType::Str && expr_idx.contains(&(i - 1)) {
                    collect_expr_reads(&self.source, *arg, registry, &mut extra);
                }
            }
            for (name, span) in extra {
                self.record_var_read(&name, span, scope_path);
            }
        }
    }

    /// Compute the scope-resolved qualified name for a command
    /// invocation at `scope_path`.
    ///
    /// Delegates the "what namespace does an unqualified call here
    /// resolve against" question to [`Self::command_resolution_namespace`]
    /// (the same rule [`command_resolution_namespace_at`] applies
    /// post-walk, so the two can't disagree), then picks the
    /// most-specific candidate from
    /// [`crate::naming::bareword_resolution_candidates`] — which already
    /// encodes Tcl's real "current namespace, then global" two-step rule
    /// for both bare names and relative names with embedded `::`
    /// (`inner::p` resolves against the current namespace before falling
    /// back to global — it is *not* unconditionally global-rooted).  A
    /// leading-`::` name is already absolute and comes back unchanged.
    ///
    /// Returned strings are intended for matching against
    /// [`super::types::ProcDef::qualified_name`] in references
    /// / document-highlight / rename providers.  The value is
    /// not authoritative for runtime dispatch — it's a
    /// candidate that lets call-site → declaration matching
    /// resolve relative call shapes.  During the walk this is the
    /// *local-first* candidate; once the whole file is walked,
    /// [`Self::finalise_invocation_resolutions`] applies Tcl's
    /// existence check and demotes the guess to the global
    /// candidate when the local one names nothing the file (or the
    /// registry) defines.
    #[must_use]
    pub fn resolve_command_qualified_name(&self, cmd_name: &str, scope_path: &[usize]) -> String {
        let ns = self.command_resolution_namespace(scope_path);
        crate::naming::bareword_resolution_candidates(&ns, cmd_name)
            .into_iter()
            .next()
            .unwrap_or_else(|| format!("::{cmd_name}"))
    }

    /// Post-walk pass: settle each invocation's `resolved_qualified_name`
    /// with Tcl's real two-step existence rule.
    ///
    /// The walk stores the *local-first* candidate (`::ns::inner::p` for an
    /// `inner::p` call inside `::ns`), but Tcl only dispatches there when
    /// that command **exists** — otherwise it falls back to the global
    /// candidate (`::inner::p`).  Existence can't be tested mid-walk: a proc
    /// body's calls resolve at call time, so a local candidate defined
    /// *later in the file* still wins (confirmed against tclsh 8.6).  This
    /// runs after the walk, when `all_procs` / `all_classes` /
    /// `command_aliases` / `renamed_commands` are complete, and demotes the
    /// guess to the global candidate when the local one names nothing known
    /// — a user definition or a registry builtin (`puts` inside a namespace
    /// resolves to `::puts`, not `::ns::puts`).
    ///
    /// When *neither* candidate is known (a cross-file proc), the
    /// local-first guess is kept: same-file consumers
    /// (`graphs`, `minify`) find no same-file definition either way, and
    /// the cross-file reference matchers treat the field as a candidate,
    /// not ground truth.
    pub(super) fn finalise_invocation_resolutions(&mut self) {
        // Populate the per-dialect builtin-name cache before splitting field
        // borrows below (`builtin_command_names` needs `&mut self`).
        let _ = self.builtin_command_names();
        let Some(builtins) = self.builtin_names.as_ref() else {
            return;
        };
        // Recorded `namespace path` declarations, cloned before borrowing
        // `result` mutably. Entries are passed as written — the shared
        // candidate builder roots a relative entry against the declaring
        // namespace (`namespace path inner` inside `::outer` means
        // `::outer::inner`, never `::inner` — tclsh-pinned; see
        // `command_resolution_candidates`).
        let paths = self.namespace_paths.clone();
        // Expose the recorded paths on the result so command-resolution
        // consumers (definition / hover / signature help) can honour a
        // `namespace path` the same way call-site settling does.
        self.result.namespace_paths.clone_from(&paths);
        // A builtin renamed away (`rename puts ::a::p`) or deleted
        // (`rename puts ""` / alias deletion) is no longer callable under
        // its original name — C raises `invalid command name` — so the
        // registry name must not count as existing. (User procs keep the
        // whole-file semantics documented above; this gates only the
        // builtin clause.)
        let renamed_away: std::collections::HashSet<String> = self
            .result
            .renamed_commands
            .values()
            .cloned()
            .chain(self.deleted_commands.keys().cloned())
            .collect();
        let result = &mut self.result;
        let (procs, classes, aliases, renames) = (
            &result.all_procs,
            &result.all_classes,
            &result.command_aliases,
            &result.renamed_commands,
        );
        let known = |qualified: &str| {
            procs.contains_key(qualified)
                || classes.contains_key(qualified)
                || aliases.contains_key(qualified)
                || renames.contains_key(qualified)
                || (builtins.contains(qualified.trim_start_matches(':'))
                    && !renamed_away.contains(qualified))
        };
        for inv in &mut result.command_invocations {
            // Absolute names are exact — the sole candidate is the name itself.
            // `None` means a background scan that skipped the scope walk; leave
            // its (empty) candidate list untouched.
            if inv.name.starts_with("::") {
                inv.resolution_candidates = vec![inv.name.clone()];
                continue;
            }
            let Some(resolved) = inv.resolved_qualified_name.clone() else {
                continue;
            };
            // The walk stored the local-first candidate (`{ns}::{name}`);
            // recover the call namespace and re-run the *shared* resolver
            // (`resolve_command_with`, the same rule the optimiser, the
            // uplevel inliner, and the VM dispatch on) now that `known` is
            // complete — with the namespace's recorded `namespace path`
            // (empty when none was declared), so a call the path would
            // catch settles to the path candidate, exactly as it
            // dispatches at run time.
            let suffix = format!("::{}", inv.name);
            let ns: String = match resolved.strip_suffix(suffix.as_str()) {
                Some("") => "::".to_owned(),
                Some(prefix) => prefix.to_owned(),
                // Not the `{ns}::{name}` shape the walk produces (an unusual
                // spelling); the settled name is then the sole candidate.
                None => {
                    inv.resolution_candidates = vec![resolved];
                    continue;
                }
            };
            let path: &[String] = paths.get(&ns).map_or(&[], Vec::as_slice);
            // Record the full ordered candidate list so a cross-document
            // consumer can re-settle this call against a *workspace-wide*
            // existence check — the local-first guess below only sees this
            // file, so a call resolving (via `namespace path`) to a proc
            // defined in another file cannot settle correctly here.
            inv.resolution_candidates =
                crate::naming::command_resolution_candidates(&ns, path, &inv.name);
            if let Some(winner) = crate::naming::resolve_command_with(&ns, path, &inv.name, &known)
                && winner != resolved
            {
                inv.resolved_qualified_name = Some(winner);
            }
            // No candidate known in this file: keep the local-first guess for
            // cross-file consumers (the reference matchers treat it, and the
            // candidate list above, as candidates rather than ground truth).
        }
    }

    /// Record a constant string assignment for `var_name` in the
    /// scope at `scope_path`.
    pub fn set_const_string(
        &mut self,
        var_name: &str,
        value: String,
        value_span: Span,
        scope_path: &[usize],
    ) {
        self.const_strings
            .entry(scope_path.to_vec())
            .or_default()
            .insert(var_name.to_string(), (value, value_span));
    }

    /// Remove constant-value knowledge for `var_name` (re-assigned
    /// dynamically).
    pub fn clear_const_string(&mut self, var_name: &str, scope_path: &[usize]) {
        if let Some(map) = self.const_strings.get_mut(scope_path) {
            map.remove(var_name);
        }
    }

    /// Look up the constant string value for `var_name`, walking
    /// the scope chain from `scope_path` outwards.
    ///
    /// Returns the nearest enclosing scope's value, or `None` if the
    /// var isn't tracked anywhere on the chain.
    #[must_use]
    pub fn lookup_const_string(&self, var_name: &str, scope_path: &[usize]) -> Option<&str> {
        self.lookup_const_string_with_span(var_name, scope_path)
            .map(|(value, _)| value)
    }

    /// Look up the constant string value for `var_name` along
    /// with the source span recorded for the defining ``set``.
    /// Used by the regex-pattern emitters to also tag the
    /// defining ``set var "..."`` value as a `RegexPattern` so
    /// semantic-token highlighting fires on the literal.
    #[must_use]
    pub fn lookup_const_string_with_span(
        &self,
        var_name: &str,
        scope_path: &[usize],
    ) -> Option<(&str, Span)> {
        for ancestor in ancestor_paths(scope_path) {
            if let Some(map) = self.const_strings.get(&ancestor)
                && let Some((value, span)) = map.get(var_name)
            {
                return Some((value.as_str(), *span));
            }
        }
        None
    }

    /// Compute the namespace string for a scope path, with a
    /// per-call cache.
    ///
    /// Walks the scope path collecting ``ScopeKind::Namespace``
    /// names, then joins them via [`normalise_qualified_name`].
    /// Caches the result on `self.ns_cache` keyed by the path.
    pub fn namespace_from_scope_path(&mut self, scope_path: &[usize]) -> String {
        if let Some(cached) = self.ns_cache.get(scope_path) {
            return cached.clone();
        }
        let mut parts: Vec<String> = Vec::new();
        let mut cursor = &self.result.global_scope;
        // The global scope itself is not a namespace component;
        // contribute its name only if we're past the root.
        for &idx in scope_path {
            let Some(child) = cursor.children.get(idx) else {
                break;
            };
            if child.kind == ScopeKind::Namespace {
                parts.push(child.name.clone());
            }
            cursor = child;
        }
        let result = if parts.is_empty() {
            "::".to_string()
        } else {
            let mut ns = "::".to_string();
            for part in parts {
                if part.starts_with("::") {
                    ns = normalise_qualified_name(&part);
                } else {
                    ns = normalise_qualified_name(&format!("{ns}::{part}"));
                }
            }
            ns
        };
        self.ns_cache.insert(scope_path.to_vec(), result.clone());
        result
    }

    /// Namespace in which an *unqualified* command invoked at
    /// `scope_path` resolves, following Tcl's command-resolution rule:
    /// the call's enclosing namespace, where a proc body resolves
    /// commands in the proc's **defining** namespace (the prefix of its
    /// qualified name) rather than its lexical parent.
    ///
    /// This differs from [`Self::namespace_from_scope_path`] (which is
    /// purely lexical and ignores proc scopes): `proc ::ns::p {...}`
    /// declared at top level has defining namespace `::ns`, so an
    /// unqualified `foo` in its body resolves to `::ns::foo` then
    /// `::foo` — even though there is no enclosing `namespace eval`.
    /// Used to scope the E002/E003 arity shadow guard to the command a
    /// call actually resolves to.
    #[must_use]
    pub(super) fn command_resolution_namespace(&self, scope_path: &[usize]) -> String {
        let mut ns = "::".to_string();
        let mut cursor = &self.result.global_scope;
        for &idx in scope_path {
            let Some(child) = cursor.children.get(idx) else {
                break;
            };
            ns = advance_command_resolution_namespace(&ns, child);
            cursor = child;
        }
        ns
    }

    /// True when `scope_path` descends through a `proc` body scope.
    ///
    /// Gates the E002/E003 arity-shadow order check.  A call inside a
    /// proc body resolves at *call* time — after the whole script has
    /// loaded — so a shadowing proc defined later in the file still
    /// silences the builtin arity check.  A top-level call (module
    /// body, `namespace eval` body, or a conditional) executes in
    /// source order during load, so only a definition that lexically
    /// precedes it shadows.
    #[must_use]
    pub(super) fn scope_path_in_proc_body(&self, scope_path: &[usize]) -> bool {
        let mut cursor = &self.result.global_scope;
        for &idx in scope_path {
            let Some(child) = cursor.children.get(idx) else {
                break;
            };
            // A method body, like a proc body, resolves its calls at call
            // time (after the whole script has loaded), so a later shadowing
            // definition still silences the arity check.
            if matches!(child.kind, ScopeKind::Proc | ScopeKind::Method) {
                return true;
            }
            cursor = child;
        }
        false
    }

    /// True when `scope_path` descends through a `TclOO` method body scope.
    ///
    /// Drives the `in_method` flag on recorded `$obj method` / `[cmd] method`
    /// dispatch sites so the W307 post-pass can apply the OO-specific
    /// suppression signals (`$self` self-reference, `my`/`self` self-dispatch
    /// with method-return inference).
    #[must_use]
    pub(super) fn scope_path_in_method_body(&self, scope_path: &[usize]) -> bool {
        let mut cursor = &self.result.global_scope;
        for &idx in scope_path {
            let Some(child) = cursor.children.get(idx) else {
                break;
            };
            if child.kind == ScopeKind::Method {
                return true;
            }
            cursor = child;
        }
        false
    }

    /// The `(class_qualified, method_name)` of the innermost `TclOO`
    /// method body enclosing `scope_path`, or `None` when the call site
    /// isn't textually inside one.
    ///
    /// Drives `next` / `nextto` arity resolution
    /// (`Analyser::queue_next_arity_candidate`): a method scope's `name`
    /// is always `"{class_qualified}::{method}"` (see
    /// `Analyser::walk_method_body`), split here on the *last* `::` —
    /// safe because a method's own simple name never itself contains
    /// `::`.
    ///
    /// A nested `proc` / lambda body between the method scope and
    /// `scope_path` resets the result to `None`: `next` only resolves
    /// inside the calling frame of the method invocation itself — a
    /// bareword `proc` defined and called from inside a method body runs
    /// in its own, unrelated frame (confirmed against tclsh 9.0.4:
    /// calling `next` from inside such a nested `proc` fails "next may
    /// only be called from inside a method"), so it must not inherit the
    /// enclosing method's context.
    #[must_use]
    pub(super) fn current_method_context(&self, scope_path: &[usize]) -> Option<(String, String)> {
        let mut cursor = &self.result.global_scope;
        let mut found: Option<(String, String)> = None;
        for &idx in scope_path {
            let Some(child) = cursor.children.get(idx) else {
                break;
            };
            match child.kind {
                ScopeKind::Method => {
                    found = child
                        .name
                        .rsplit_once("::")
                        .map(|(cls, method)| (cls.to_string(), method.to_string()));
                }
                ScopeKind::Proc => found = None,
                ScopeKind::Global | ScopeKind::Namespace | ScopeKind::Uplevel => {}
            }
            cursor = child;
        }
        found
    }

    /// Record a variable read for go-to-definition / find-references.
    ///
    /// Looks for the variable in the scope at `scope_path`; falls
    /// back to the global scope for ``::``- and ``static::``-prefixed
    /// names. W210 (read-before-set) is emitted by the SSA-based
    /// pass elsewhere — this helper only records the reference
    /// span.
    pub fn record_var_read(&mut self, name: &str, read_span: Span, scope_path: &[usize]) {
        let base_name = normalise_var_name(name);
        if base_name.is_empty() {
            return;
        }
        // An `$arr(idx)` read records the element index on the array var.
        let element = split_array_name(name)
            .1
            .filter(|e| !e.is_empty())
            .map(ToString::to_string);

        // Local scope first.
        let path = scope_path.to_vec();
        let base_owned = base_name.to_string();
        if let Some(scope) = scope_at_mut(&mut self.result.global_scope, &path)
            && let Some(var) = scope.variables.get_mut(&base_owned)
        {
            var.references.push(read_span);
            if let Some(e) = element {
                var.array_indices.insert(e);
            }
            return;
        }

        // Cross-rule variables — fall back to global scope.
        if base_owned.starts_with("::") || base_owned.starts_with("static::") {
            if let Some(var) = self.result.global_scope.variables.get_mut(&base_owned) {
                var.references.push(read_span);
                if let Some(e) = element {
                    var.array_indices.insert(e);
                }
            } else if let Some(captured) = self.capture_global_reads.as_mut() {
                // Isolated per-item body: the enclosing global scope is empty
                // here, so a qualified read that would resolve against it in a
                // whole-file analyse is captured for the aggregator to replay on
                // the shell's real global scope.  `name` (not `base_owned`)
                // preserves any `arr(idx)` element for the replay.
                captured.push((name.to_string(), read_span));
            }
        }
    }

    /// The variable names defined so far in the scope at `scope_path` —
    /// the walk-time in-scope candidate set for the W212 / W215
    /// "did you mean…?" suggestions.  Owned strings so callers can keep
    /// the list across later scope mutations.
    #[must_use]
    pub(in crate::analyser) fn scope_variable_names(&self, scope_path: &[usize]) -> Vec<String> {
        scope_at(&self.result.global_scope, scope_path)
            .map(|s| s.variables.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Record a variable definition in the scope at `scope_path`.
    ///
    /// New definitions are inserted into the scope's `variables` map
    /// and registered into ``result.all_variables`` keyed by
    /// ``"<scope_name>::<base_name>"``.  Re-defining an existing
    /// variable does not overwrite — it only escalates the
    /// `warn_if_unused` flag.
    pub fn define_var(
        &mut self,
        name: &str,
        tok: Token,
        scope_path: &[usize],
        warn_if_unused: bool,
        definition_span: Option<Span>,
    ) {
        let base_name = normalise_var_name(name);
        if base_name.is_empty() {
            return;
        }
        let base_owned = base_name.to_string();
        let span = definition_span.unwrap_or(tok.span);
        // A `set arr(idx) …` definition records the element index on the array.
        let element = split_array_name(name)
            .1
            .filter(|e| !e.is_empty())
            .map(ToString::to_string);

        let path = scope_path.to_vec();
        // First definition of this variable → the W215 reachability check
        // fires here.
        // Done with a short immutable borrow before the mutable scope handle.
        let is_first_def = scope_at(&self.result.global_scope, &path)
            .is_some_and(|s| !s.variables.contains_key(base_name));
        if is_first_def && !self.suppress_w215 {
            self.emit_w215_unreachable_name(base_name, element.as_deref(), tok, span, &path);
        }
        let Some(scope) = scope_at_mut(&mut self.result.global_scope, &path) else {
            return;
        };

        if let Some(existing) = scope.variables.get_mut(&base_owned) {
            // Re-definition (`set x` twice) does not overwrite the original
            // declaration span, but its span is recorded as a reference so
            // find-references / rename see every assignment, and it escalates
            // the unused flag.  Array indices accumulate.
            //
            // The last-reference check makes re-definition idempotent per
            // token span: a write site is recorded once even when both its
            // dedicated handler (`incr` / `append` / `dict update`) and the
            // registry `VarWrite`-role walk (`handle_var_binding_command`)
            // bind it — the two run back-to-back for the same word, and a
            // duplicated reference becomes a duplicated (hence overlapping)
            // rename edit downstream.
            if span != existing.definition_span && existing.references.last() != Some(&span) {
                existing.references.push(span);
            }
            if warn_if_unused {
                existing.warn_if_unused = true;
            }
            if let Some(e) = element {
                existing.array_indices.insert(e);
            }
            if let Some(global) = self
                .result
                .all_variables
                .get_mut(&format!("{}::{base_owned}", scope.name))
            {
                *global = scope.variables[&base_owned].clone();
            }
        } else {
            let mut indices = std::collections::BTreeSet::new();
            if let Some(e) = element {
                indices.insert(e);
            }
            let var = VarDef {
                name: base_owned.clone(),
                definition_span: span,
                references: Vec::new(),
                warn_if_unused,
                array_indices: indices,
                link_target: None,
            };
            scope.variables.insert(base_owned.clone(), var.clone());
            let key = format!("{}::{base_owned}", scope.name);
            self.result.all_variables.insert(key, var);
        }
    }

    /// Record that the local `name` in the scope at `scope_path` aliases the
    /// namespace/global cell `target` (a `global` / `variable` / `namespace
    /// upvar` binding), so Find-References / Rename can unify every alias of the
    /// same cell.  A no-op if the variable wasn't defined (call after
    /// [`Self::define_var`]).
    pub fn set_var_link_target(&mut self, name: &str, scope_path: &[usize], target: String) {
        let base = crate::naming::normalise_var_name(name).to_string();
        if let Some(scope) = scope_at_mut(&mut self.result.global_scope, scope_path)
            && let Some(v) = scope.variables.get_mut(&base)
        {
            v.link_target = Some(target);
        }
    }

    /// **W215.** Emit when a variable's runtime name (or array element
    /// index) cannot be reached via `$`-substitution, even though it can
    /// be created/read via `set` / `info exists` / `upvar`.  The runtime
    /// name applies Tcl backslash substitution unless the word was
    /// braced (`STR`), since `{...}` preserves every byte verbatim.
    ///
    /// An unreachable name is usually a stray-delimiter typo of an
    /// ordinary nearby name (`set "a}b" 1` for `set ab 1`), so the
    /// message carries a "; did you mean 'X'?" suffix naming the closest
    /// variable already defined in the same scope, when one sits within
    /// the length-scaled edit budget.
    fn emit_w215_unreachable_name(
        &mut self,
        base_name: &str,
        element: Option<&str>,
        tok: Token,
        site_span: Span,
        scope_path: &[usize],
    ) {
        use super::types::{Diagnostic, Severity};
        use crate::naming::is_brace_substitutable;
        use std::borrow::Cow;

        let braced = tok.kind == TokenType::Str;
        let subst = |s: &str| -> String {
            if s.contains('\\') && !braced {
                tcl_lexer::backslash_subst(s).into_owned()
            } else {
                s.to_string()
            }
        };

        let runtime_name = subst(base_name);
        // The `${…}` delimiting rule is dialect-dependent (Tcl 9 tracks
        // nested braces and `\X` pairs; the 8.x family stops at the first
        // literal `}`) — resolve it from the active dialect's lexer config.
        let nesting = self.lexer_config().braced_var.nests();
        if !is_brace_substitutable(&runtime_name, nesting) {
            let detail = if runtime_name.contains('}') {
                if nesting {
                    "the brace form ``${name}`` ends at the first unbalanced ``}`` not preceded by ``\\`` (and the bare form stops at the first non-word character)"
                } else {
                    "the brace form ``${name}`` ends at the first ``}`` (and the bare form stops at the first non-word character)"
                }
            } else if runtime_name.ends_with('\\') {
                "the brace form ``${name}`` would read the trailing ``\\`` as the start of a 2-char escape and run out of input -- missing close-brace"
            } else if runtime_name.contains('{') {
                "the brace form ``${name}`` has unbalanced ``{`` and runs out of input -- missing close-brace"
            } else {
                "the brace form ``${name}`` cannot match this name"
            };
            let mut message = format!(
                "variable name ``{runtime_name}`` is not reachable via $-substitution; \
                 it can still be created/read via ``set name`` / ``[set \"name\"]`` / \
                 ``info exists`` / ``upvar``, but {detail}"
            );
            let candidates = self.scope_variable_names(scope_path);
            let suggestions = crate::text::suggest_similar(
                &runtime_name,
                candidates
                    .iter()
                    .map(String::as_str)
                    .filter(|name| *name != runtime_name),
                1,
                crate::text::scaled_max_distance_strict(&runtime_name),
            );
            if let Some(best) = suggestions.first() {
                use std::fmt::Write as _;
                let _ = write!(message, "; did you mean '{best}'?");
            }
            self.result.diagnostics.push(Diagnostic {
                code: DiagCode::W215,
                span: site_span,
                message,
                severity: Severity::Warning,
                fixes: Vec::new(),
            });
        }

        if let Some(elem) = element {
            let runtime_element: Cow<str> = if elem.contains('\\') && !braced {
                Cow::Owned(tcl_lexer::backslash_subst(elem).into_owned())
            } else {
                Cow::Borrowed(elem)
            };
            if runtime_element.contains(')') {
                self.result.diagnostics.push(Diagnostic {
                    code: DiagCode::W215,
                    span: site_span,
                    message: "array element index contains ')'; the element can be created \
                              and read via ``set arr(idx) ...`` / ``[set \"arr(idx)\"]``, but \
                              is not reachable via $-substitution (``$arr(idx)`` reads up \
                              to the first ``)`` and stops there)"
                        .to_string(),
                    severity: Severity::Warning,
                    fixes: Vec::new(),
                });
            }
        }
    }

    /// Walk the scope tree depth-first, yielding the scope at
    /// `scope_path` and every descendant in declaration order.
    ///
    /// Collects the paths into a `Vec` rather than returning a lazy
    /// iterator — simpler borrowing for the same iteration order.
    #[must_use]
    pub fn walk_scopes_from(&self, scope_path: &[usize]) -> Vec<Vec<usize>> {
        let mut out = Vec::new();
        let Some(start) = scope_at(&self.result.global_scope, scope_path) else {
            return out;
        };
        walk_scopes_helper(start, scope_path, &mut out);
        out
    }
}

fn walk_scopes_helper(scope: &Scope, path: &[usize], out: &mut Vec<Vec<usize>>) {
    out.push(path.to_vec());
    for (i, child) in scope.children.iter().enumerate() {
        let mut child_path = path.to_vec();
        child_path.push(i);
        walk_scopes_helper(child, &child_path, out);
    }
}

// Substituted / expr `$var` read collection

/// Extract the base variable name from a `Var` token's source span:
/// `$name` / `${name}` → `name`, and `$arr(idx)` → `arr` (the array
/// index is dropped so element reads attribute to the base array
/// variable).
///
/// Handles the lexer's `Var`-token span convention, which excludes
/// the closing `}` of a braced reference (`${x}` spans `${x`), so the
/// trailing brace is stripped only when present.  The array-index
/// suffix is dropped for the unbraced form only — inside `${…}` the
/// parentheses are part of a literal name.  The recorded base name is
/// re-normalised by [`normalise_var_name`] in `record_var_read`; this
/// helper keeps that contract so reads attribute to the right slot.
fn var_name_from_span(source: &str, span: Span) -> Option<&str> {
    let (s, e) = (span.start() as usize, span.end() as usize);
    if s > e || e > source.len() {
        return None;
    }
    let text = &source[s..e];
    let rest = text.strip_prefix('$')?;
    let name = if let Some(inner) = rest.strip_prefix('{') {
        // Braced `${name}` — the span may omit the closing brace.
        inner.strip_suffix('}').unwrap_or(inner)
    } else {
        // Unbraced `$arr(idx)` — keep the index too, so `record_var_read`
        // (via `split_array_name`) can record the element on the array var
        // (it normalises to the base name for the reference itself).
        rest
    };
    if name.is_empty() { None } else { Some(name) }
}

/// Inner content text + base offset of a wrapper token (`[…]` /
/// `{…}`): the delimiter is normally a single leading byte, skipped via
/// `content_offset` — not hardcoded, since a synthetic recovery token
/// (e.g. `Analyser::recover_stray_close_bracket`'s virtual `Cmd` token)
/// has no real opener in the source and sets `content_offset` to `0`.
/// The span excludes the closing delimiter.
fn inner_of(source: &str, tok: Token) -> Option<(&str, u32)> {
    let off = u32::from(tok.content_offset);
    let (s, e) = ((tok.span.start() + off) as usize, tok.span.end() as usize);
    if s > e || e > source.len() {
        return None;
    }
    Some((&source[s..e], tok.span.start() + off))
}

/// Collect `$var` reads inside a command-substitution token,
/// recursing into nested substitutions, expr arguments, and
/// same-scope (plain) body arguments.  Structural-body arguments
/// (proc / namespace / oo) introduce a new scope and are skipped.
fn collect_cmd_subst_reads(
    source: &str,
    cmd_tok: Token,
    registry: &tcl_registry::CommandRegistry,
    out: &mut Vec<(String, Span)>,
) {
    let Some((inner, base)) = inner_of(source, cmd_tok) else {
        return;
    };
    for cmd in crate::segmenter::segment_commands_with_offset(inner, base) {
        collect_script_command_reads(source, &cmd, registry, out);
    }
}

/// Record reads for one command appearing inside a substitution:
/// top-level `$var` tokens, plus recursion into nested
/// substitutions / expr args / plain bodies.
fn collect_script_command_reads(
    source: &str,
    cmd: &crate::segmenter::SegmentedCommand,
    registry: &tcl_registry::CommandRegistry,
    out: &mut Vec<(String, Span)>,
) {
    let head_span = cmd.argv.first().map(|t| t.span);
    for tok in &cmd.all_tokens {
        if tok.kind == tcl_lexer::TokenType::Var && Some(tok.span) != head_span {
            if let Some(name) = var_name_from_span(source, tok.span) {
                out.push((name.to_owned(), tok.span));
            }
        } else if tok.kind == tcl_lexer::TokenType::Cmd {
            collect_cmd_subst_reads(source, *tok, registry, out);
        }
    }
    let cmd_name = cmd.texts.first().map_or("", String::as_str);
    let post: Vec<&str> = cmd.texts.iter().skip(1).map(String::as_str).collect();
    let expr_idx: std::collections::HashSet<usize> = registry
        .arg_indices_for_role(cmd_name, &post, tcl_registry::ArgRole::Expr)
        .into_iter()
        .collect();
    let body_idx: std::collections::HashSet<usize> = registry
        .arg_indices_for_role(cmd_name, &post, tcl_registry::ArgRole::Body)
        .into_iter()
        .collect();
    let structural = registry
        .get(cmd_name)
        .is_some_and(|s| s.body_kind == tcl_registry::BodyKind::Structural);
    for (i, arg) in cmd.argv.iter().enumerate().skip(1) {
        if arg.kind != tcl_lexer::TokenType::Str {
            continue;
        }
        let pidx = i - 1;
        if expr_idx.contains(&pidx) {
            collect_expr_reads(source, *arg, registry, out);
        } else if body_idx.contains(&pidx) && !structural {
            // Plain body inside a substitution — same scope, and the
            // main walk never reached it.
            if let Some((inner, base)) = inner_of(source, *arg) {
                for sub in crate::segmenter::segment_commands_with_offset(inner, base) {
                    collect_script_command_reads(source, &sub, registry, out);
                }
            }
        }
    }
}

/// Collect `$var` reads inside a braced `expr` argument: every
/// `Var` token plus any command substitution nested in the
/// expression.
fn collect_expr_reads(
    source: &str,
    expr_tok: Token,
    registry: &tcl_registry::CommandRegistry,
    out: &mut Vec<(String, Span)>,
) {
    let Some((inner, base)) = inner_of(source, expr_tok) else {
        return;
    };
    let Ok(tokens) = tcl_lexer::Lexer::new(inner).tokenise_all() else {
        return;
    };
    for tok in tokens {
        let abs = Span::new(tok.span.start() + base, tok.span.end() + base);
        match tok.kind {
            tcl_lexer::TokenType::Var => {
                if let Some(name) = var_name_from_span(source, abs) {
                    out.push((name.to_owned(), abs));
                }
            }
            tcl_lexer::TokenType::Cmd => {
                collect_cmd_subst_reads(
                    source,
                    Token::new(tcl_lexer::TokenType::Cmd, abs),
                    registry,
                    out,
                );
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::TokenType;

    fn span(start: u32, end: u32) -> Span {
        Span::new(start, end)
    }

    fn make_analyser_with_namespace(ns_name: &str) -> Analyser {
        let mut a = Analyser::new();
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, ns_name));
        a
    }

    #[test]
    fn current_scope_defaults_to_global() {
        let a = Analyser::new();
        assert_eq!(a.current_scope().name, "::");
        assert_eq!(a.current_scope().kind, ScopeKind::Global);
    }

    #[test]
    fn current_scope_follows_path() {
        let mut a = make_analyser_with_namespace("ns1");
        a.current_scope_path = vec![0];
        assert_eq!(a.current_scope().name, "ns1");
        assert_eq!(a.current_scope().kind, ScopeKind::Namespace);
    }

    #[test]
    fn set_then_lookup_const_string() {
        let mut a = Analyser::new();
        a.set_const_string("x", "hello".to_string(), span(0, 5), &[]);
        assert_eq!(a.lookup_const_string("x", &[]), Some("hello"));
    }

    #[test]
    fn lookup_const_string_walks_chain() {
        // x defined in global; lookup from a child scope finds it.
        let mut a = make_analyser_with_namespace("ns1");
        a.set_const_string("x", "hello".to_string(), span(0, 5), &[]);
        assert_eq!(a.lookup_const_string("x", &[0]), Some("hello"));
    }

    #[test]
    fn lookup_const_string_inner_shadows_outer() {
        let mut a = make_analyser_with_namespace("ns1");
        a.set_const_string("x", "outer".to_string(), span(0, 5), &[]);
        a.set_const_string("x", "inner".to_string(), span(10, 15), &[0]);
        assert_eq!(a.lookup_const_string("x", &[0]), Some("inner"));
        assert_eq!(a.lookup_const_string("x", &[]), Some("outer"));
    }

    #[test]
    fn clear_const_string_removes_entry() {
        let mut a = Analyser::new();
        a.set_const_string("x", "hello".to_string(), span(0, 5), &[]);
        a.clear_const_string("x", &[]);
        assert_eq!(a.lookup_const_string("x", &[]), None);
    }

    #[test]
    fn lookup_const_string_returns_none_for_unknown() {
        let a = Analyser::new();
        assert_eq!(a.lookup_const_string("x", &[]), None);
    }

    #[test]
    fn namespace_from_scope_path_global_returns_root() {
        let mut a = Analyser::new();
        assert_eq!(a.namespace_from_scope_path(&[]), "::");
    }

    #[test]
    fn namespace_from_scope_path_single_relative() {
        let mut a = make_analyser_with_namespace("ns1");
        assert_eq!(a.namespace_from_scope_path(&[0]), "::ns1");
    }

    #[test]
    fn namespace_from_scope_path_absolute_rebases() {
        let mut a = make_analyser_with_namespace("outer");
        // Add an absolute child namespace.
        a.result.global_scope.children[0]
            .children
            .push(Scope::new(ScopeKind::Namespace, "::abs"));
        // Path [0, 0] should rebase at ::abs, not nest under outer.
        assert_eq!(a.namespace_from_scope_path(&[0, 0]), "::abs");
    }

    #[test]
    fn namespace_from_scope_path_caches_result() {
        let mut a = make_analyser_with_namespace("ns1");
        let _ = a.namespace_from_scope_path(&[0]);
        assert!(a.ns_cache.contains_key(&vec![0_usize]));
    }

    #[test]
    fn namespace_from_scope_path_skips_proc_scopes() {
        // proc scopes don't contribute to the namespace path.
        let mut a = Analyser::new();
        let mut ns = Scope::new(ScopeKind::Namespace, "ns1");
        ns.children
            .push(Scope::new(ScopeKind::Proc, "::ns1::myproc"));
        a.result.global_scope.children.push(ns);
        assert_eq!(a.namespace_from_scope_path(&[0, 0]), "::ns1");
    }

    /// The Method-scope namespace rule: a `TclOO` method (flagged) resolves
    /// globally; a snit/itcl method (unflagged) resolves in the class's
    /// defining namespace, like a proc.
    #[test]
    fn method_scope_namespace_honours_oo_global_resolution() {
        let flagged = {
            let mut m = Scope::new(ScopeKind::Method, "::pkg::C::m");
            m.oo_global_resolution = true;
            m
        };
        assert_eq!(
            advance_command_resolution_namespace("::pkg", &flagged),
            "::"
        );
        let unflagged = Scope::new(ScopeKind::Method, "::pkg::C::m");
        assert_eq!(
            advance_command_resolution_namespace("::pkg", &unflagged),
            "::pkg::C",
        );
    }

    /// tclsh 8.6.16 / 9.0.4-pinned (G1 shape): a bare call in a `TclOO`
    /// method body dispatches the GLOBAL helper, not the class's
    /// defining-namespace helper — method bodies run in the object's
    /// namespace and never search the class's namespace.
    #[test]
    fn tcloo_method_call_settles_to_the_global_helper() {
        let src = "namespace eval ::pkg {\n\
                       proc helper {} { return PKGHELPER }\n\
                       oo::class create C { method m {} { helper } }\n\
                   }\n\
                   proc ::helper {} { return GLOBALHELPER }\n";
        let mut a = Analyser::new();
        let analysis = a.analyse(src, "tcl8.6");
        let inv = analysis
            .command_invocations
            .iter()
            .find(|i| i.name == "helper")
            .expect("method-body helper invocation recorded");
        assert_eq!(
            inv.resolved_qualified_name.as_deref(),
            Some("::helper"),
            "TclOO method bodies resolve bare calls globally (tclsh-pinned)",
        );
    }

    /// A builtin renamed away is not callable under its original name
    /// (tclsh-pinned: C raises `invalid command name`), so settlement must
    /// not demote a namespace-local guess to the dead builtin name.
    #[test]
    fn renamed_away_builtin_no_longer_settles_calls() {
        let src = "rename puts ::a::p\n\
                   namespace eval ::ns { puts x }\n";
        let mut a = Analyser::new();
        let analysis = a.analyse(src, "tcl8.6");
        let inv = analysis
            .command_invocations
            .iter()
            .find(|i| i.name == "puts" && i.resolved_qualified_name.as_deref() != Some("::a::p"))
            .expect("namespaced puts invocation recorded");
        assert_eq!(
            inv.resolved_qualified_name.as_deref(),
            Some("::ns::puts"),
            "the dead builtin name must not win; the local-first guess stays",
        );
    }

    fn diag_codes(source: &str, dialect: &str) -> Vec<String> {
        let mut a = Analyser::new();
        a.analyse(source, dialect)
            .diagnostics
            .iter()
            .map(|d| d.code.to_string())
            .collect()
    }

    // finalise_invocation_resolutions — Tcl's existence-checked two-step
    // resolution rule, applied post-walk.  Every case below is pinned
    // against tclsh 8.6 (PR #924 review): the local-first candidate wins
    // only when that command exists by the end of the file; otherwise the
    // call falls back to the global candidate.

    /// The `resolved_qualified_name` recorded for the invocation whose raw
    /// call text is `name`.
    fn resolved_for(source: &str, name: &str) -> Option<String> {
        let mut a = Analyser::new();
        let analysis = a.analyse(source, "tcl8.6");
        analysis
            .command_invocations
            .iter()
            .find(|inv| inv.name == name)
            .unwrap_or_else(|| panic!("no `{name}` invocation recorded"))
            .resolved_qualified_name
            .clone()
    }

    #[test]
    fn relative_qualified_call_falls_back_to_global_when_local_absent() {
        // tclsh8.6: `inner::p` inside `outer` dispatches ::inner::p when
        // ::outer::inner::p does not exist.
        let src = "namespace eval ::inner {}\nproc ::inner::p {} {}\nnamespace eval outer { proc caller {} { inner::p } }\n";
        assert_eq!(resolved_for(src, "inner::p").as_deref(), Some("::inner::p"));
    }

    #[test]
    fn relative_qualified_call_prefers_local_defined_later_in_file() {
        // tclsh8.6: a proc body's calls resolve at call time, so the local
        // candidate wins even when its `proc` runs after the caller was
        // defined — the walk order must not leak into the resolution.
        let src = concat!(
            "namespace eval ::inner {}\n",
            "proc ::inner::p {} {}\n",
            "namespace eval outer { proc caller {} { inner::p } }\n",
            "namespace eval outer { namespace eval inner {} ; proc inner::p {} {} }\n",
        );
        assert_eq!(
            resolved_for(src, "inner::p").as_deref(),
            Some("::outer::inner::p")
        );
    }

    #[test]
    fn bare_call_falls_back_to_global_when_local_absent() {
        let src = "proc ::g {} {}\nnamespace eval ns2 { proc caller {} { g } }\n";
        assert_eq!(resolved_for(src, "g").as_deref(), Some("::g"));
    }

    #[test]
    fn bare_call_prefers_local_defined_later_in_file() {
        let src = concat!(
            "proc ::g {} {}\n",
            "namespace eval ns2 { proc caller {} { g } }\n",
            "namespace eval ns2 { proc g {} {} }\n",
        );
        assert_eq!(resolved_for(src, "g").as_deref(), Some("::ns2::g"));
    }

    #[test]
    fn builtin_call_inside_namespace_resolves_global() {
        // tclsh8.6: `puts` inside a namespace is the global builtin — the
        // local-first guess `::ns3::puts` names nothing, so the settled
        // resolution is `::puts` (as it was before the scope-aware walk).
        let src = "namespace eval ns3 { proc caller {} { puts hi } }\n";
        assert_eq!(resolved_for(src, "puts").as_deref(), Some("::puts"));
    }

    #[test]
    fn builtin_shadowed_by_local_proc_stays_local() {
        // tclsh8.6: a namespace-local `puts` shadows the builtin for
        // unqualified calls from inside that namespace.
        let src = "namespace eval ns3 { proc puts {msg} {} ; proc caller {} { puts hi } }\n";
        assert_eq!(resolved_for(src, "puts").as_deref(), Some("::ns3::puts"));
    }

    #[test]
    fn unknown_in_both_namespaces_keeps_local_first_guess() {
        // Neither ::ns4::mystery::call nor ::mystery::call is defined in
        // this file (a cross-file proc, or a typo W123 reports) — keep the
        // local-first candidate so cross-file consumers still see the
        // Tcl-priority guess.
        let src = "namespace eval ns4 { proc caller {} { mystery::call } }\n";
        assert_eq!(
            resolved_for(src, "mystery::call").as_deref(),
            Some("::ns4::mystery::call")
        );
    }

    #[test]
    fn w215_fires_for_unreachable_quoted_name() {
        assert!(diag_codes("set \"a}b\" 1", "tcl").contains(&"W215".to_string()));
    }

    #[test]
    fn w215_quiet_for_braced_escaped_name() {
        // `{a\}b}` is braced, so `\}` is preserved verbatim and the
        // runtime name `a\}b` IS brace-substitutable.
        assert!(!diag_codes("set {a\\}b} 1", "tcl").contains(&"W215".to_string()));
    }

    #[test]
    fn w215_fires_for_array_index_with_paren() {
        assert!(diag_codes("set arr(a)b) 1", "tcl").contains(&"W215".to_string()));
    }

    #[test]
    fn w215_quiet_for_normal_name() {
        assert!(!diag_codes("set normal 1", "tcl").contains(&"W215".to_string()));
    }

    #[test]
    fn w215_suggests_close_in_scope_variable() {
        // TP: `"a}b"` is a stray-brace typo of the already-defined `ab`
        // (edit distance 1) — the message names it.
        let mut a = Analyser::new();
        let r = a.analyse("set ab 1\nset \"a}b\" 2\n", "tcl");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::W215)
            .expect("W215");
        assert!(
            d.message.ends_with("; did you mean 'ab'?"),
            "expected an 'ab' suggestion: {:?}",
            d.message
        );
    }

    #[test]
    fn w215_no_suggestion_when_nothing_is_close() {
        // FP guard: no defined variable sits within the edit budget —
        // the message keeps its plain form.
        let mut a = Analyser::new();
        let r = a.analyse("set totally 1\nset \"qq}zz\" 2\n", "tcl");
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::W215)
            .expect("W215");
        assert!(
            !d.message.contains("did you mean"),
            "no suggestion expected: {:?}",
            d.message
        );
    }

    #[test]
    fn w215_quiet_for_backslash_continued_param_list() {
        // Issue #743: a parameter list wrapped across lines with `\`. Tcl
        // list-parses the braced param list, so `ddrtol\<newline>ddatol` is two
        // parameters (`ddrtol`, `ddatol`), not a single `ddrtol\` name — no
        // W215 unreachable-name warning should fire.
        let src = "proc p {a ddrtol\\\n        ddatol} { list $a $ddrtol $ddatol }";
        assert!(!diag_codes(src, "tcl").contains(&"W215".to_string()));

        // The reported form: a TclOO `method` with a wrapped parameter list.
        let method_src = "oo::class create C {\n  method Fdjac2 {funct ifree ddrtol\\\n      ddatol} { list $ddrtol $ddatol }\n}\n";
        assert!(!diag_codes(method_src, "tcl").contains(&"W215".to_string()));
    }

    #[test]
    fn w116_fires_for_stub_shadowing_builtin() {
        let src = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub set {a:var b}\n# tcl-lsp: stubs-end\n";
        assert!(diag_codes(src, "tcl").contains(&"W116".to_string()));
    }

    #[test]
    fn w117_fires_for_stub_expr_shadowing_builtin() {
        let src = "# tcl-lsp: stubs-begin\n# tcl-lsp: stub expr-func sin 1\n# tcl-lsp: stubs-end\n";
        assert!(diag_codes(src, "tcl").contains(&"W117".to_string()));
    }

    #[test]
    fn w127_fires_for_value_outside_closed_set() {
        let src = "when HTTP_REQUEST { HTTP::version \"2.0\" }";
        assert!(diag_codes(src, "f5-irules").contains(&"W127".to_string()));
    }

    #[test]
    fn w127_quiet_for_allowed_value() {
        let src = "when HTTP_REQUEST { HTTP::version \"1.1\" }";
        assert!(!diag_codes(src, "f5-irules").contains(&"W127".to_string()));
    }

    #[test]
    fn define_var_inserts_into_scope_and_all_variables() {
        let mut a = Analyser::new();
        let tok = Token::new(TokenType::Esc, span(0, 4));
        a.define_var("x", tok, &[], true, None);
        assert!(a.result.global_scope.variables.contains_key("x"));
        assert!(a.result.all_variables.contains_key("::::x"));
        let v = &a.result.global_scope.variables["x"];
        assert!(v.warn_if_unused);
        assert_eq!(v.definition_span, span(0, 4));
    }

    #[test]
    fn define_var_existing_only_escalates_warn_flag() {
        let mut a = Analyser::new();
        let tok = Token::new(TokenType::Esc, span(0, 4));
        a.define_var("x", tok, &[], false, None);
        assert!(!a.result.global_scope.variables["x"].warn_if_unused);
        a.define_var("x", tok, &[], true, None);
        assert!(a.result.global_scope.variables["x"].warn_if_unused);
        // Definition span unchanged — first definition wins.
        assert_eq!(
            a.result.global_scope.variables["x"].definition_span,
            span(0, 4)
        );
    }

    #[test]
    fn define_var_uses_explicit_definition_span() {
        let mut a = Analyser::new();
        let tok = Token::new(TokenType::Esc, span(0, 4));
        a.define_var("x", tok, &[], true, Some(span(10, 15)));
        assert_eq!(
            a.result.global_scope.variables["x"].definition_span,
            span(10, 15)
        );
    }

    #[test]
    fn record_var_read_local_scope() {
        let mut a = Analyser::new();
        let tok = Token::new(TokenType::Esc, span(0, 4));
        a.define_var("x", tok, &[], true, None);
        a.record_var_read("x", span(20, 21), &[]);
        assert_eq!(
            a.result.global_scope.variables["x"].references,
            vec![span(20, 21)]
        );
    }

    #[test]
    fn record_var_read_global_fallback_for_qualified_name() {
        // Var defined in global scope, read from a child namespace
        // via qualified name — should fall back to global.
        let mut a = make_analyser_with_namespace("ns1");
        let tok = Token::new(TokenType::Esc, span(0, 4));
        a.define_var("::x", tok, &[], true, None);
        a.record_var_read("::x", span(20, 21), &[0]);
        assert_eq!(
            a.result.global_scope.variables["::x"].references,
            vec![span(20, 21)]
        );
    }

    #[test]
    fn record_var_read_no_match_silent() {
        let mut a = Analyser::new();
        // No var defined; read does nothing.
        a.record_var_read("x", span(20, 21), &[]);
        // No panic, no references recorded anywhere.
        assert!(a.result.global_scope.variables.is_empty());
    }

    #[test]
    fn walk_scopes_from_root_visits_all() {
        let mut a = Analyser::new();
        let mut ns = Scope::new(ScopeKind::Namespace, "ns1");
        ns.children.push(Scope::new(ScopeKind::Proc, "::ns1::p"));
        a.result.global_scope.children.push(ns);
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "ns2"));
        let paths = a.walk_scopes_from(&[]);
        assert_eq!(paths, vec![vec![], vec![0], vec![0, 0], vec![1]]);
    }
}
