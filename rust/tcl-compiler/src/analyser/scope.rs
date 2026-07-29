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

use std::collections::{HashMap, HashSet};

use tcl_core_types::DiagCode;
use tcl_lexer::{Span, Token, TokenType};

use crate::naming::{normalise_qualified_name, normalise_var_name, split_array_name};

use super::state::Analyser;
use super::types::{ClassDef, ProcDef, Scope, ScopeKind, VarDef};

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
///
/// `part` is a *written* namespace name — canonicalise it once (colon runs
/// collapse to one separator; a trailing run drops, as C's namespace-name
/// lookup does) and then join with one exact `::`.  The accumulated `ns` is a
/// *constructed* key and is never re-parsed: a `::`-run inside it is a
/// legitimately colon-named segment (`namespace eval :`), which a
/// concat-then-normalise would collapse into its parent (issue #934).
fn join_namespace(ns: &str, part: &str) -> String {
    if part.starts_with("::") {
        return normalise_qualified_name(part);
    }
    let segs = crate::naming::qualifier_segments_owned(part);
    if segs.is_empty() {
        // `namespace eval {}` (and an all-separator name) is the namespace
        // itself resolved relative — the empty name is the *global* namespace
        // in C, but as a relative component it contributes nothing.
        return ns.to_owned();
    }
    let joined = segs.join("::");
    if ns == "::" || ns.is_empty() {
        format!("::{joined}")
    } else {
        format!("{ns}::{joined}")
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
            // The defining namespace is the qualified name's *holder* — via
            // the command-name join and the construction-inverse split, so a
            // lone-colon or trailing-separator proc name derives the right
            // prefix (#934: `proc :` at the root defines in `::`, never in a
            // phantom namespace named `:`).
            let qualified = crate::naming::qualify(ns, &child.name);
            let (holder, _tail) = crate::naming::key_holder_and_tail(&qualified);
            if holder.is_empty() {
                "::".to_string()
            } else {
                holder.to_string()
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

/// Whether an unqualified command at `byte_offset` also resolves against
/// `::oo::Helpers`, the one fixed namespace real Tcl's `TclOO` implementation
/// always searches for a method body's bare command calls — the runtime
/// counterpart to [`Scope::oo_global_resolution`]'s static `"::"`-only
/// approximation, which [`advance_command_resolution_namespace`] can't
/// otherwise represent (a single accumulated namespace string has no room
/// for a second, non-lexical search member).
///
/// A real-world idiom installs a proc directly into `::oo::Helpers`
/// (`proc ::oo::Helpers::classvar {...} {...}`, the documented `TclOO`
/// Tricks pattern nico-robert/ticklecharts uses) to make it bare-callable
/// from every method body in the program — this is what lets
/// [`tcl_lsp_core::references::invocation_references_named`]'s namespace
/// gate recognise such a call site as a genuine reference to that proc,
/// alongside the ordinary `call_ns == target_ns` case (issue #923 idx 56,
/// main audit wave).
///
/// Same traversal as [`command_resolution_namespace_at`] (so the two can
/// never disagree about which scope is innermost), tracking whether the
/// *last* namespace-resolution-affecting scope kind visited on the way
/// down was an `oo_global_resolution` method — a `namespace eval` (or any
/// other scope kind) nested inside a method body leaves the method's
/// object-context resolution behind, exactly like [`ScopeKind::Namespace`]
/// resets nothing in `advance_command_resolution_namespace` but a
/// `Proc`/`Method`/`Uplevel` scope does.
#[must_use]
pub fn innermost_scope_reaches_oo_helpers(root: &Scope, byte_offset: u32) -> bool {
    let mut reaches = false;
    let mut cursor = root;
    loop {
        let next = cursor.children.iter().find(|c| {
            c.body_span
                .is_some_and(|s| s.start() <= byte_offset && byte_offset < s.end())
        });
        let Some(child) = next else { break };
        reaches = match child.kind {
            ScopeKind::Namespace | ScopeKind::Global => reaches,
            ScopeKind::Method => child.oo_global_resolution,
            ScopeKind::Proc | ScopeKind::Uplevel => false,
        };
        cursor = child;
    }
    reaches
}

/// The [`VarDef`] a `::`-qualified variable reference names — `base_name`
/// declared directly in the namespace whose fully qualified, `::`-rooted
/// resolution path is `target_ns` — the single-table, no-searching lookup
/// real Tcl performs for a qualified variable name (tclsh 9.0.4 / 8.6.16
/// -verified: unlike command resolution, `$other::v` never searches
/// enclosing namespaces or falls back to global; exactly one namespace is
/// consulted, whichever the qualifier names).
///
/// A real namespace can be `namespace eval`-reopened any number of times in
/// any number of lexically unrelated places (even nested inside unrelated
/// procs — `namespace eval` always addresses the one real namespace tree
/// regardless of where it's textually written), so no single [`Scope`] tree
/// node can be assumed to hold every variable a qualified reference might
/// mean. This walks the *whole* tree, accumulating each node's
/// resolution-namespace with the same [`advance_command_resolution_namespace`]
/// rule [`command_resolution_namespace_at`] uses, and consults the
/// `variables` table of every [`ScopeKind::Namespace`] / [`ScopeKind::Global`]
/// node whose accumulated namespace equals `target_ns` — deliberately never a
/// [`ScopeKind::Proc`] / [`ScopeKind::Method`] node's own local table, even
/// when *its* command-resolution namespace happens to equal `target_ns`: a
/// proc's locals are never the namespace's cells (a plain `set` local has no
/// relation to a same-named namespace variable; only an explicit
/// `variable`/`global` link would, and that link already resolves through
/// the ordinary scope-chain walk, not this path).
#[must_use]
pub fn lookup_var_in_namespace<'a>(
    root: &'a Scope,
    target_ns: &str,
    base_name: &str,
) -> Option<&'a VarDef> {
    fn walk<'a>(ns: &str, node: &'a Scope, target: &str, name: &str) -> Option<&'a VarDef> {
        if ns == target
            && matches!(node.kind, ScopeKind::Namespace | ScopeKind::Global)
            && let Some(v) = node.variables.get(name)
        {
            return Some(v);
        }
        node.children.iter().find_map(|child| {
            let child_ns = advance_command_resolution_namespace(ns, child);
            walk(&child_ns, child, target, name)
        })
    }
    walk("::", root, target_ns, base_name)
}

/// Bundles the field-disjoint borrows [`Analyser::finalise_invocation_resolutions`]'s
/// call-site "is `qualified` known, for a call at `call_off`?" predicate
/// needs, and the small body/enclosing-definition/liveness helpers it's
/// built from — extracted so the loop that builds and consults it doesn't
/// blow that function's line budget. `A` is left generic over the
/// alias-map value type (only `contains_key` is used) so this doesn't need
/// to name `command_aliases`'s value type explicitly.
struct KnownPredicateCtx<'a, A> {
    builtins: &'a HashSet<String>,
    renamed_away: &'a HashSet<String>,
    procs: &'a HashMap<String, ProcDef>,
    classes: &'a HashMap<String, ClassDef>,
    aliases: &'a HashMap<String, A>,
    renames: &'a HashMap<String, String>,
    alias_offsets: &'a HashMap<String, u32>,
    rename_offsets: &'a HashMap<String, u32>,
    deleted_commands: &'a HashMap<String, u32>,
    reachable_call_offsets: &'a HashMap<String, u32>,
}

impl<A> KnownPredicateCtx<'_, A> {
    /// Whether byte offset `off` falls inside any proc/class body recorded
    /// so far — a local restatement of
    /// `AnalysisResult::offset_is_inside_any_definition_body` over the
    /// field-disjoint borrows above, since a method call on `result` (the
    /// whole struct) would conflict with the `&mut
    /// result.command_invocations` borrow the caller's loop holds.
    fn offset_inside_any_body(&self, off: u32) -> bool {
        self.procs
            .values()
            .any(|p| p.body_span.start() <= off && off < p.body_span.end())
            || self
                .classes
                .values()
                .any(|c| c.body_span.start() <= off && off < c.body_span.end())
    }

    /// The qualified name of the innermost proc/class body containing
    /// `off`, mirroring `AnalysisResult::enclosing_definition_qualified_name`
    /// for the same borrow-splitting reason as `offset_inside_any_body`.
    fn enclosing_definition(&self, off: u32) -> Option<&str> {
        self.procs
            .values()
            .map(|p| (p.qualified_name.as_str(), p.body_span))
            .chain(
                self.classes
                    .values()
                    .map(|c| (c.qualified_name.as_str(), c.body_span)),
            )
            .filter(|(_, span)| span.start() <= off && off < span.end())
            .min_by_key(|(_, span)| span.end() - span.start())
            .map(|(qn, _)| qn)
    }

    /// Whether `qualified`'s fact — a proc/class definition, a `rename`
    /// target, or an `interp alias` target, established at `fact_off` — is
    /// still live *for a call at `call_off`*: no `rename NAME {}` / `interp
    /// alias {} NAME {}` deletion of `qualified` itself has been recorded
    /// *after* that offset (issue #973: a proc/class/rename/alias target
    /// that was later renamed away must not still count as known —
    /// calling it fails "invalid command name" in real Tcl, confirmed
    /// against tclsh 8.6.14). Mirrors `fact_superseded_by_deletion` in
    /// `diagnostics/validity.rs` (the arity resolver's answer to the same
    /// "most recent fact: live definition or deletion?" question) rather
    /// than re-deriving it: a name deleted and then re-established under
    /// the same name (a fresh `proc`, `rename`, or `interp alias`) is live
    /// again, so only a deletion that postdates this specific fact's own
    /// establishing offset disqualifies it. `deleted_commands` stores only
    /// the last-seen deletion offset per name, which — since the walk
    /// visits statements in source order — is always the most recent one.
    ///
    /// Call-site and conditional-body aware the same way
    /// [`Analyser::fact_live_for_call`] is (the W123 pass's answer to the
    /// identical question) — a namespaced local candidate must not lose to
    /// the global one just because *some later* deletion exists, when this
    /// specific call runs before it; issue #1009 Codex review: `proc bar
    /// {}`, `namespace eval foo { proc bar {}; proc caller {} { bar } }`,
    /// `foo::caller`, `rename foo::bar {}` still resolves `bar` (called
    /// from `foo::caller`, before the rename) to `::foo::bar`, not the
    /// global `::bar` — confirmed against tclsh 8.6.14 — because
    /// `foo::caller`'s own top-level invocation already ran before the
    /// rename.
    ///
    /// The enclosing definition need not be called at the top level
    /// *itself* — [`Analyser::reachable_call_offsets`] answers the
    /// transitive question (issue #1015), so an arbitrarily deep chain of
    /// enclosing definitions bottoming out at a real top-level call counts,
    /// while a mutual-recursion cycle no top-level call enters does not.
    fn live_for_call(&self, qualified: &str, fact_off: u32, call_off: u32) -> bool {
        let Some(&del_off) = self.deleted_commands.get(qualified) else {
            return true;
        };
        if del_off <= fact_off {
            return true;
        }
        if self.offset_inside_any_body(del_off) {
            return true;
        }
        if !self.offset_inside_any_body(call_off) {
            return call_off <= del_off;
        }
        self.enclosing_definition(call_off)
            .and_then(|qn| self.reachable_call_offsets.get(qn))
            .is_some_and(|&t| t < del_off)
    }

    fn known(&self, qualified: &str, call_off: u32) -> bool {
        self.procs
            .get(qualified)
            .is_some_and(|p| self.live_for_call(qualified, p.name_span.start(), call_off))
            || self
                .classes
                .get(qualified)
                .is_some_and(|c| self.live_for_call(qualified, c.name_span.start(), call_off))
            || (self.aliases.contains_key(qualified)
                && self
                    .alias_offsets
                    .get(qualified)
                    .is_none_or(|&off| self.live_for_call(qualified, off, call_off)))
            || (self.renames.contains_key(qualified)
                && self
                    .rename_offsets
                    .get(qualified)
                    .is_none_or(|&off| self.live_for_call(qualified, off, call_off)))
            || (self.builtins.contains(qualified.trim_start_matches(':'))
                && !self.renamed_away.contains(qualified))
    }
}

/// The [`VarDef`] a fully qualified `target` (always `::`-rooted; see
/// [`crate::analyser::handlers::Analyser::handle_global_command`]'s
/// convention) names, when it was declared by a *literal* `set` whose own
/// spelling already carried some or all of its namespace qualification
/// (`set ::tolComp val`, or `set Bar::baz val` written inside `namespace
/// eval Foo`) — [`define_var`] never re-qualifies a name it's given, it
/// only strips a `$`/`${…}` wrapper and an array index
/// ([`normalise_var_name`]), so such a write's stored key is `target`
/// itself (or a suffix of it), never the bare tail
/// [`lookup_var_in_namespace`] expects.
///
/// Restricted to the same [`ScopeKind::Namespace`] / [`ScopeKind::Global`]
/// node kinds as [`lookup_var_in_namespace`] (a proc-local `set ::x val`
/// stores under the *proc's* own table, not reachable here — same
/// documented trade-off `lookup_var_in_scope_chain` already accepts for the
/// general case; issue #923 idx 68 only needs the realistic top-level /
/// namespace-body shape the audit's own repro exercises). Matches by exact
/// `VarDef::name` equality against `target` rather than a table lookup,
/// since the stored key can be any literal spelling that *resolves* to
/// `target`, not necessarily `target`'s own exact text.
#[must_use]
fn lookup_var_by_literal_qualified_name<'a>(root: &'a Scope, target: &str) -> Option<&'a VarDef> {
    if matches!(root.kind, ScopeKind::Namespace | ScopeKind::Global)
        && let Some(v) = root.variables.values().find(|v| v.name == target)
    {
        return Some(v);
    }
    root.children
        .iter()
        .find_map(|child| lookup_var_by_literal_qualified_name(child, target))
}

/// The [`VarDef`] a fully qualified `target` names, trying both storage
/// conventions a direct (non-alias) declaration can use: the bare-tail
/// table key [`lookup_var_in_namespace`] expects (a `variable`-declared
/// namespace cell, or an unqualified top-level `set`), then — if that
/// misses — the literal-verbatim key [`lookup_var_by_literal_qualified_name`]
/// expects (a `set` whose own spelling already carried its qualification).
/// The single entry point `tcl_lsp_core::definition::linked_var_reference_spans`
/// needs (issue #923 idx 68) to fold a `global`/`variable`/`namespace upvar`
/// alias's target back to its canonical cell regardless of which way the
/// cell itself was spelled.
#[must_use]
pub fn lookup_var_by_qualified_name<'a>(root: &'a Scope, target: &str) -> Option<&'a VarDef> {
    let (target_ns, base_name) = crate::naming::key_holder_and_tail(target);
    lookup_var_in_namespace(root, target_ns, base_name)
        .or_else(|| lookup_var_by_literal_qualified_name(root, target))
}

/// The fully qualified name (`::ns::name`) a *direct* variable
/// declaration at `def_span` would carry, if some `global` / `variable`
/// / `namespace upvar` alias elsewhere pointed at it — the inverse of
/// [`lookup_var_in_namespace`]: that function resolves a known qualified
/// name to its declaring [`VarDef`]; this one goes the other way,
/// resolving an already-known declaration to the qualified name an alias
/// would need to name it by.
///
/// Used to check whether a plain declaration with no `link_target` of
/// its own (`link_target: None` — it isn't an alias, so it was never
/// given one) is nonetheless the *canonical cell* an alias in another
/// scope names via its own `link_target` (issue #923 idx 68, main audit
/// wave: a top-level `set tolComp` / `set ::tolComp`, aliased inside a
/// proc via `global tolComp`, needs Find-References/Rename queried from
/// *either* side to reach both — querying from the alias already finds
/// the cell via [`lookup_var_by_qualified_name`], but the reverse
/// direction, querying from the cell itself, had no way to find the alias
/// back without this).
///
/// Same whole-tree walk and namespace-accumulation rule as
/// [`lookup_var_in_namespace`], restricted to the same
/// [`ScopeKind::Namespace`] / [`ScopeKind::Global`] node kinds — matches
/// by `definition_span` (a real declaration's span is unique to it;
/// callers must not pass an empty span, which several unrelated
/// declaration-less seeds can share, per [`VarDef::definition_span`]'s
/// own doc) rather than by name, since the caller doesn't know which
/// scope holds `def_span` ahead of time.
#[must_use]
pub fn qualified_name_for_var_decl(root: &Scope, def_span: tcl_lexer::Span) -> Option<String> {
    fn walk(ns: &str, node: &Scope, def_span: tcl_lexer::Span) -> Option<String> {
        if matches!(node.kind, ScopeKind::Namespace | ScopeKind::Global)
            && let Some(v) = node
                .variables
                .values()
                .find(|v| v.definition_span == def_span)
        {
            // A literal write's own spelling can already be absolute
            // (`set ::tolComp val` stores `name == "::tolComp"` verbatim —
            // `define_var` never re-qualifies what it's given, see
            // `lookup_var_by_literal_qualified_name`'s doc) — prefixing
            // again would double it (`"::::tolComp"`, matching nothing).
            // Only a *bare* tail (the `variable`/`global` convention, or an
            // unqualified literal `set`) needs `ns` prepended.
            return Some(if v.name.starts_with("::") {
                v.name.clone()
            } else if ns == "::" {
                format!("::{}", v.name)
            } else {
                format!("{ns}::{}", v.name)
            });
        }
        node.children.iter().find_map(|child| {
            let child_ns = advance_command_resolution_namespace(ns, child);
            walk(&child_ns, child, def_span)
        })
    }
    walk("::", root, def_span)
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
    /// Registry builtin names renamed away (`rename puts ::a::p`) or
    /// deleted (`rename puts ""` / an `interp alias` deletion) anywhere in
    /// the file — no longer callable under their original name (C raises
    /// `invalid command name`), so the registry name must not count as
    /// existing. Gates only [`Self::finalise_invocation_resolutions`]'s
    /// registry-builtin clause; user procs/classes keep that function's
    /// whole-file semantics (a proc/class's own deletion is instead gated
    /// per qualified name, via `deleted_commands` directly).
    fn renamed_away_builtin_names(&self) -> std::collections::HashSet<String> {
        self.result
            .renamed_commands
            .values()
            .cloned()
            .chain(self.deleted_commands.keys().cloned())
            .collect()
    }

    /// Build [`Analyser::reachable_call_offsets`]: the earliest offset at
    /// which each resolved qualified name is *provably reached* by the
    /// file's own top-level execution.
    ///
    /// The base case is a call whose own site is not inside any proc/class
    /// body — that call runs where it is written. The recursive case is a
    /// call inside some definition `E`'s body: it runs whenever `E` runs, so
    /// it inherits `E`'s own earliest reachable offset (issue #1015:
    /// `proc helper {}`, `proc inner {} { helper }`, `proc outer {} { inner
    /// }`, `outer`, `rename helper {}` — tclsh8.6/9.0 run this clean,
    /// because `outer`'s top-level call reaches `helper` two bodies deep,
    /// before the rename).
    ///
    /// Computed as a monotone least-fixpoint over the call graph rather
    /// than a memoised recursion: offsets only ever decrease, so the loop
    /// terminates, and a mutual-recursion cycle with no top-level entry
    /// simply never acquires a value (`None` — unreachable) instead of
    /// recursing forever.
    ///
    /// Read by [`Self::finalise_invocation_resolutions`]'s `live_for_call`
    /// and by [`Self::fact_live_for_call`] as the "was this call's enclosing
    /// definition itself reached before the deletion" escape hatch (issue
    /// #1009 Codex review, generalised by #1015).
    fn compute_reachable_call_offsets(&self) -> HashMap<String, u32> {
        let mut reachable: HashMap<String, u32> = HashMap::new();
        // Call-graph edges: `enclosing definition -> callee`, for every
        // invocation whose own site sits inside a body.
        let mut body_edges: Vec<(&str, &str)> = Vec::new();
        for inv in &self.result.command_invocations {
            let Some(qualified) = inv.resolved_qualified_name.as_deref() else {
                continue;
            };
            let call_off = inv.range.start();
            if !self.result.offset_is_inside_any_definition_body(call_off) {
                reachable
                    .entry(qualified.to_string())
                    .and_modify(|off| *off = (*off).min(call_off))
                    .or_insert(call_off);
                continue;
            }
            if let Some(enclosing) = self.result.enclosing_definition_qualified_name(call_off) {
                body_edges.push((enclosing, qualified));
            }
        }
        loop {
            let mut changed = false;
            for &(enclosing, callee) in &body_edges {
                let Some(&reached) = reachable.get(enclosing) else {
                    continue;
                };
                match reachable.get_mut(callee) {
                    Some(off) if *off <= reached => {}
                    Some(off) => {
                        *off = reached;
                        changed = true;
                    }
                    None => {
                        reachable.insert(callee.to_string(), reached);
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        reachable
    }

    pub(super) fn finalise_invocation_resolutions(&mut self) {
        // Populate the per-dialect builtin-name cache before splitting field
        // borrows below (`builtin_command_names` needs `&mut self`).
        let _ = self.builtin_command_names();
        let Some(builtins) = self.builtin_names.as_ref() else {
            return;
        };
        // `&'static str`, not a borrow of `self` — safe to read after the
        // field-splitting `result` borrow below, unlike an `&self` method
        // call (which is why the math-function existence check the
        // mathfunc branch uses is a free function, not `Analyser::dialect`
        // plus an instance method).
        let dialect = self.dialect();
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
        // Earliest top-level (non-body) call-site offset per resolved
        // qualified name, from the invocations the walk has already
        // recorded — computed before the mutable loop below so
        // `live_for_call`'s body-call escape hatch (issue #1009 Codex
        // review) can consult it, and cached on `self` so the later W123 /
        // const-dispatch / variable-command passes' `Self::fact_live_for_call`
        // calls reuse the same map instead of rebuilding it.
        self.reachable_call_offsets = self.compute_reachable_call_offsets();
        let renamed_away = self.renamed_away_builtin_names();
        let deleted_commands = &self.deleted_commands;
        let rename_offsets = &self.rename_offsets;
        let reachable_call_offsets = &self.reachable_call_offsets;
        let result = &mut self.result;
        let (procs, classes, aliases, renames) = (
            &result.all_procs,
            &result.all_classes,
            &result.command_aliases,
            &result.renamed_commands,
        );
        let alias_offsets = &result.alias_offsets;
        // See `KnownPredicateCtx` (module level, above `impl Analyser`) for
        // the call-site + conditional-body-aware "is `qualified` known?"
        // predicate this builds — extracted out of this function to stay
        // within the line budget.
        let known_ctx = KnownPredicateCtx {
            builtins,
            renamed_away: &renamed_away,
            procs,
            classes,
            aliases,
            renames,
            alias_offsets,
            rename_offsets,
            deleted_commands,
            reachable_call_offsets,
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
            // A math-function invocation's resolved name carries the fixed
            // `tcl::mathfunc` dispatch segment, never the calling namespace —
            // the generic one-hop `{ns}::{name}` strip just below assumes
            // `resolved` is exactly `{callingNamespace}::{name}`, which is
            // false here (it would misparse `::tcl::mathfunc::sin` as if
            // `::tcl::mathfunc` were the calling namespace, silently
            // mis-resolving to an unrelated global command that happens to
            // share the bare tail name — a same-file `proc sin {…}` would
            // hijack `expr {sin(x)}`). `is_mathfunc_call` — set once, at
            // record time, never re-derived by guessing at the resolved
            // string's shape — routes these through the real two-candidate
            // rule instead: the caller's own `tcl::mathfunc` (honouring its
            // `namespace path`, exactly like the VM's own
            // `resolve_command_fqn` does for every command lookup, math
            // functions included), else the global one — `known` plus, for
            // the global candidate only, genuine built-in membership (a
            // local slot is never a built-in; only the true global one
            // ever is).
            if inv.is_mathfunc_call {
                let mathfunc_suffix = format!("::tcl::mathfunc::{}", inv.name);
                let ns: String = match resolved.strip_suffix(mathfunc_suffix.as_str()) {
                    Some("") => "::".to_owned(),
                    Some(prefix) => prefix.to_owned(),
                    // The walk always produces this shape for a mathfunc
                    // invocation; treat an unrecognised one conservatively,
                    // matching the generic path's own fallback below.
                    None => {
                        inv.resolution_candidates = vec![resolved];
                        continue;
                    }
                };
                let path: &[String] = paths.get(&ns).map_or(&[], Vec::as_slice);
                let rel = format!("tcl::mathfunc::{}", inv.name);
                let candidates = crate::naming::command_resolution_candidates(&ns, path, &rel);
                inv.resolution_candidates.clone_from(&candidates);
                let global = format!("::tcl::mathfunc::{}", inv.name);
                let call_off = inv.range.start();
                let winner = candidates.into_iter().find(|c| {
                    known_ctx.known(c, call_off)
                        || (*c == global
                            && crate::tcl_expr_eval::is_known_mathfunc_in_dialect(
                                &inv.name, dialect,
                            ))
                });
                if let Some(w) = winner
                    && w != resolved
                {
                    inv.resolved_qualified_name = Some(w);
                }
                continue;
            }
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
            let call_off = inv.range.start();
            let known_here = |c: &str| known_ctx.known(c, call_off);
            if let Some(winner) =
                crate::naming::resolve_command_with(&ns, path, &inv.name, &known_here)
                && winner != resolved
            {
                inv.resolved_qualified_name = Some(winner);
            }
            // No candidate known in this file: keep the local-first guess for
            // cross-file consumers (the reference matchers treat it, and the
            // candidate list above, as candidates rather than ground truth).
        }
        // The constant-`$cmd` dispatch sites (M7) are *not* settled here:
        // their value facts come from the compiler's flow-sensitive value
        // model, which needs the CFG/SSA `CompilationUnit` — see
        // `settle_const_dispatches` in the diagnostics phase (issue #945
        // faults 1–2: the lexical constant map this pass once read is
        // last-write-wins across `if`/loop joins and discards the defining
        // literal's writable span).
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
        // Track whether this binding's *current* value dominates uses after
        // an `if`/`try` join. A conditional write (`conditional_depth > 0`)
        // does not; a straight-line (depth-0) write does and re-establishes
        // dominance over any earlier conditional one (Codex review, PR
        // #1020 — see `nondominating_consts` / `lookup_dominating_const_string`).
        if self.conditional_depth > 0 {
            self.nondominating_consts
                .entry(scope_path.to_vec())
                .or_default()
                .insert(var_name.to_string());
        } else if let Some(set) = self.nondominating_consts.get_mut(scope_path) {
            set.remove(var_name);
        }
    }

    /// Remove constant-value knowledge for `var_name` (re-assigned
    /// dynamically).
    pub fn clear_const_string(&mut self, var_name: &str, scope_path: &[usize]) {
        if let Some(map) = self.const_strings.get_mut(scope_path) {
            map.remove(var_name);
        }
        if let Some(set) = self.nondominating_consts.get_mut(scope_path) {
            set.remove(var_name);
        }
    }

    /// Like [`Self::lookup_const_string`], but only returns a value that
    /// **dominates** this use site — i.e. whose nearest-enclosing-scope
    /// binding was last written straight-line (not inside an `if`/`try`
    /// body). A binding poisoned in [`Self::nondominating_consts`] at the
    /// scope where it is found yields `None`: the value is branch-dependent,
    /// so identity resolution (`source`/`rename` targets) must abstain
    /// rather than pick the last-written branch (Codex review, PR #1020).
    /// The nearest binding still wins — a poisoned inner binding shadows an
    /// outer dominating one, exactly as Tcl variable scoping would.
    #[must_use]
    pub fn lookup_dominating_const_string(
        &self,
        var_name: &str,
        scope_path: &[usize],
    ) -> Option<&str> {
        for ancestor in ancestor_paths(scope_path) {
            if let Some(map) = self.const_strings.get(&ancestor)
                && let Some((value, _)) = map.get(var_name)
            {
                let poisoned = self
                    .nondominating_consts
                    .get(&ancestor)
                    .is_some_and(|s| s.contains(var_name));
                return if poisoned { None } else { Some(value.as_str()) };
            }
        }
        None
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

    /// The constant-string value recorded for `base_name` in the namespace
    /// whose fully-qualified, `::`-rooted resolution path is `target_ns`
    /// (issue #923 idx 116) — the `const_strings` analogue of
    /// [`lookup_var_in_namespace`], needed because a namespace-qualified
    /// reference (`$lexical::body`) must resolve against wherever that
    /// namespace's own `set` lives, not the lexical ancestor chain
    /// [`Self::lookup_const_string_with_span`] walks. `const_strings` is
    /// keyed by scope *path* (`Vec<usize>`), not a [`super::types::Scope`]
    /// reference, so — unlike `lookup_var_in_namespace`'s pure namespace-
    /// string accumulation — this walk must also track the running
    /// child-index path as it descends. Deliberately never a
    /// [`super::types::ScopeKind::Proc`] / [`super::types::ScopeKind::Method`]
    /// node's own table, even when its command-resolution namespace happens
    /// to equal `target_ns`: a proc-local `set` is never the namespace's
    /// own cell, mirroring `lookup_var_in_namespace`'s identical guard.
    #[must_use]
    pub fn lookup_const_string_in_namespace(
        &self,
        target_ns: &str,
        base_name: &str,
    ) -> Option<(&str, Span)> {
        fn walk<'a>(
            ns: &str,
            node: &'a super::types::Scope,
            path: &mut Vec<usize>,
            target: &str,
            name: &str,
            const_strings: &'a std::collections::HashMap<
                Vec<usize>,
                std::collections::HashMap<String, (String, Span)>,
            >,
        ) -> Option<(&'a str, Span)> {
            if ns == target
                && matches!(
                    node.kind,
                    super::types::ScopeKind::Namespace | super::types::ScopeKind::Global
                )
                && let Some(map) = const_strings.get(path.as_slice())
                && let Some((value, span)) = map.get(name)
            {
                return Some((value.as_str(), *span));
            }
            for (i, child) in node.children.iter().enumerate() {
                path.push(i);
                let child_ns = advance_command_resolution_namespace(ns, child);
                if let Some(hit) = walk(&child_ns, child, path, target, name, const_strings) {
                    return Some(hit);
                }
                path.pop();
            }
            None
        }
        let mut path: Vec<usize> = Vec::new();
        walk(
            "::",
            &self.result.global_scope,
            &mut path,
            target_ns,
            base_name,
            &self.const_strings,
        )
    }

    /// Record `set VAR [interp create ...]`'s resolved interpreter-domain
    /// `key` as `var_name`'s value in the scope at `scope_path` (issue
    /// #923 idx 9) — the interpreter-value-flow analogue of
    /// [`Self::set_const_string`], scope-chain-aware for the same reason:
    /// two unrelated procs binding the same variable name to different
    /// interpreters must never collide.
    pub fn set_interp_var_binding(&mut self, var_name: &str, key: String, scope_path: &[usize]) {
        self.interp_var_bindings
            .entry(scope_path.to_vec())
            .or_default()
            .insert(var_name.to_string(), key);
    }

    /// Remove interpreter-binding knowledge for `var_name` (reassigned to
    /// something else — the reassigned value stops resolving as an
    /// interpreter handle, exactly as [`Self::clear_const_string`] does
    /// for a reassigned constant string).
    pub fn clear_interp_var_binding(&mut self, var_name: &str, scope_path: &[usize]) {
        if let Some(map) = self.interp_var_bindings.get_mut(scope_path) {
            map.remove(var_name);
        }
    }

    /// Look up the interpreter-domain key `var_name` was bound to,
    /// walking the scope chain from `scope_path` outwards — the
    /// interpreter-value-flow analogue of [`Self::lookup_const_string`].
    #[must_use]
    pub fn lookup_interp_var_binding(&self, var_name: &str, scope_path: &[usize]) -> Option<&str> {
        for ancestor in ancestor_paths(scope_path) {
            if let Some(map) = self.interp_var_bindings.get(&ancestor)
                && let Some(key) = map.get(var_name)
            {
                return Some(key.as_str());
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
            // One join per written segment — `join_namespace` canonicalises
            // the written part and appends with an exact separator, so a
            // namespace legitimately named `:` never collapses (#934).
            let mut ns = "::".to_string();
            for part in parts {
                ns = join_namespace(&ns, &part);
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
    /// names, and — under a Tcl 8.x dialect (TIP 278) — for a bare
    /// name read at a namespace frame whose namespace has no such
    /// variable but the global namespace does. W210 (read-before-set)
    /// is emitted by the SSA-based pass elsewhere — this helper only
    /// records the reference span.
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
        } else if !base_owned.contains("::")
            && self.result.ns_var_global_fallback()
            && scope_at(&self.result.global_scope, &path)
                .is_some_and(|s| s.kind == ScopeKind::Namespace)
            && let Some(var) = self.result.global_scope.variables.get_mut(&base_owned)
        {
            // TIP 278 — at a **namespace frame**, Tcl 8.x resolves a bare
            // undefined name against the global namespace (never an
            // intermediate namespace; 9.0 dropped even the global hop), so
            // attach the read to the global cell for find-references /
            // rename.  A namespace-local declaration (`variable v`) was
            // caught by the local-scope branch above, so reaching here means
            // the namespace table genuinely lacks the name — exactly the C
            // fallback condition (8.6 `tclVar.c` `TclLookupSimpleVar`).
            var.references.push(read_span);
            if let Some(e) = element {
                var.array_indices.insert(e);
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
        walk_scopes_helper(start, scope_path, &mut out, 0);
        out
    }
}

/// Depth cap for [`walk_scopes_helper`]'s recursion over nested
/// (namespace / proc / method) [`Scope`] children — issue #996.
/// Transitively bounded today via `analyser::commands::MAX_BODY_DEPTH`
/// (the analyser's own recursive descent, which builds this `Scope` tree
/// in the first place, already caps its own nesting at 256), capped here
/// independently for defence-in-depth and consistency with every other
/// full-tree walker in this crate.
const MAX_SCOPE_WALK_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(256);

fn walk_scopes_helper(scope: &Scope, path: &[usize], out: &mut Vec<Vec<usize>>, depth: u32) {
    if MAX_SCOPE_WALK_DEPTH.exceeded(depth) {
        return;
    }
    out.push(path.to_vec());
    for (i, child) in scope.children.iter().enumerate() {
        let mut child_path = path.to_vec();
        child_path.push(i);
        walk_scopes_helper(child, &child_path, out, depth + 1);
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
pub(super) fn inner_of(source: &str, tok: Token) -> Option<(&str, u32)> {
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
        // `rename`'s own OLD argument (`puts`, line 0) is now itself a
        // recorded, correctly-resolved-to-the-still-live-builtin reference
        // (issue #923 idx 39) — an earlier "puts" entry this test must not
        // mistake for the *call* site (`puts x`, line 1) it actually means
        // to check. The call site is always the *last* "puts"-named
        // invocation in source order.
        let inv = analysis
            .command_invocations
            .iter()
            .rev()
            .find(|i| i.name == "puts")
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

    /// Regression coverage for issue #996: `walk_scopes_helper` recurses
    /// once per nested [`Scope`] child, with no depth cap of its own
    /// before this fix. Transitively bounded to
    /// `analyser::commands::MAX_BODY_DEPTH` (256) by the analyser pass
    /// that builds the `Scope` tree today, so this is defence-in-depth /
    /// consistency with every other full-tree walker in this crate, not a
    /// currently-reproducible crash. 2000 levels is comfortably past this
    /// new cap; the assertion is that `walk_scopes_from` returns at all,
    /// not what it returns.
    #[test]
    fn deeply_nested_scopes_survive_walk_scopes_from() {
        const DEPTH: usize = 2000;
        let mut a = Analyser::new();
        let mut leaf = Scope::new(ScopeKind::Proc, "::leaf");
        for i in 0..DEPTH {
            let mut wrapper = Scope::new(ScopeKind::Namespace, format!("ns{i}"));
            wrapper.children.push(leaf);
            leaf = wrapper;
        }
        a.result.global_scope.children.push(leaf);
        let paths = a.walk_scopes_from(&[]);
        assert!(!paths.is_empty());
    }

    fn var(name: &str, def_span: Span) -> VarDef {
        VarDef {
            name: name.to_string(),
            definition_span: def_span,
            references: Vec::new(),
            warn_if_unused: false,
            array_indices: std::collections::BTreeSet::new(),
            link_target: None,
        }
    }

    #[test]
    fn lookup_var_in_namespace_finds_var_in_matching_top_level_namespace() {
        // TP — the base case both mined findings (idx=107, idx=115) reduce
        // to: `namespace eval ::simple { variable v ... }`, then `$::simple::v`
        // referenced elsewhere.
        let mut root = Scope::new(ScopeKind::Global, "::");
        let mut ns_a = Scope::new(ScopeKind::Namespace, "::A");
        ns_a.variables
            .insert("v".to_string(), var("v", span(10, 11)));
        root.children.push(ns_a);

        let found = lookup_var_in_namespace(&root, "::A", "v");
        assert_eq!(found.map(|v| v.definition_span), Some(span(10, 11)));
        assert!(lookup_var_in_namespace(&root, "::A", "missing").is_none());
    }

    #[test]
    fn lookup_var_in_namespace_accumulates_through_nested_namespaces() {
        // TP — a namespace written 2+ levels deep must accumulate through
        // every enclosing `namespace eval`, exactly like command resolution
        // (`advance_command_resolution_namespace`, reused verbatim here).
        let mut root = Scope::new(ScopeKind::Global, "::");
        let mut ns_a = Scope::new(ScopeKind::Namespace, "A");
        let mut ns_b = Scope::new(ScopeKind::Namespace, "B");
        ns_b.variables
            .insert("v".to_string(), var("v", span(20, 21)));
        ns_a.children.push(ns_b);
        root.children.push(ns_a);

        assert_eq!(
            lookup_var_in_namespace(&root, "::A::B", "v").map(|v| v.definition_span),
            Some(span(20, 21))
        );
        // The intermediate namespace's own (empty) table must not be
        // mistaken for the leaf's.
        assert!(lookup_var_in_namespace(&root, "::A", "v").is_none());
    }

    #[test]
    fn lookup_var_in_namespace_finds_var_defined_in_a_reopened_block() {
        // TP — the generalisation the mined findings' fix direction calls
        // for: a namespace can be `namespace eval`-reopened any number of
        // times (even lexically unrelated ones), so a single Scope tree
        // node must never be assumed to hold every variable a qualified
        // reference might mean. Two independent `::A` blocks here; only the
        // second declares `v`.
        let mut root = Scope::new(ScopeKind::Global, "::");
        root.children.push(Scope::new(ScopeKind::Namespace, "::A"));
        let mut ns_a_reopened = Scope::new(ScopeKind::Namespace, "::A");
        ns_a_reopened
            .variables
            .insert("v".to_string(), var("v", span(30, 31)));
        root.children.push(ns_a_reopened);

        assert_eq!(
            lookup_var_in_namespace(&root, "::A", "v").map(|v| v.definition_span),
            Some(span(30, 31))
        );
    }

    #[test]
    fn lookup_var_in_namespace_skips_proc_locals_even_when_namespace_matches() {
        // FP guard — a proc's own defining ("command-resolution") namespace
        // can coincide with a target namespace path (any proc's does, by
        // construction), but its *local* variable table is never the
        // namespace's cells: a plain `set v 1` inside `proc ::A::f` has no
        // relation to a same-named `::A::v` unless linked via `variable`/
        // `global` (which resolves through the ordinary scope-chain walk,
        // not this namespace-table lookup). Only `f`'s *own* locals exist
        // here — `::A` itself declares nothing — so a hit would prove the
        // guard is missing.
        let mut root = Scope::new(ScopeKind::Global, "::");
        let mut ns_a = Scope::new(ScopeKind::Namespace, "::A");
        let mut proc_f = Scope::new(ScopeKind::Proc, "f");
        proc_f
            .variables
            .insert("v".to_string(), var("v", span(40, 41)));
        ns_a.children.push(proc_f);
        root.children.push(ns_a);

        assert!(
            lookup_var_in_namespace(&root, "::A", "v").is_none(),
            "a proc-local variable must never satisfy a namespace-qualified lookup"
        );
    }

    #[test]
    fn lookup_var_in_namespace_returns_none_for_unknown_namespace() {
        // TN — a target namespace that appears nowhere in the tree.
        let root = Scope::new(ScopeKind::Global, "::");
        assert!(lookup_var_in_namespace(&root, "::Nope", "v").is_none());
    }

    #[test]
    fn lookup_var_in_namespace_finds_global_scope_variable() {
        // TP — the degenerate `target_ns == "::"` case: the root scope
        // itself is `::`, so an absolute `$::v` reference must resolve
        // directly against the root's own variable table.
        let mut root = Scope::new(ScopeKind::Global, "::");
        root.variables.insert("v".to_string(), var("v", span(0, 1)));
        assert_eq!(
            lookup_var_in_namespace(&root, "::", "v").map(|v| v.definition_span),
            Some(span(0, 1))
        );
    }

    // `finalise_invocation_resolutions`'s local-vs-global candidate choice,
    // Codex PR #1014 review comment #1 (`scope.rs:463`): the "known"
    // predicate compared only the final deletion offset against the
    // candidate's own establishing offset, never the call site, so a
    // namespaced local call textually *before* a later unconditional
    // deletion wrongly lost to the global candidate. Confirmed against
    // tclsh 8.6.14 throughout.

    #[test]
    fn local_call_before_later_deletion_resolves_local_not_global_codex_1009() {
        // TP (the confirmed regression): `foo::caller`'s own top-level
        // invocation runs before `rename foo::bar {}`, so its `bar` call
        // must still resolve to the local `::foo::bar`, not the global
        // `::bar` — confirmed against tclsh 8.6.14 (prints "local").
        let mut a = Analyser::new();
        let src = "proc bar {} { return global }\nnamespace eval foo {\n    proc bar {} { return local }\n    proc caller {} { return [bar] }\n}\nfoo::caller\nrename foo::bar {}\n";
        let r = a.analyse(src, "tcl8.6");
        let bar_call = r
            .command_invocations
            .iter()
            .find(|i| i.name == "bar" && !i.indirect)
            .expect("bar call recorded");
        assert_eq!(
            bar_call.resolved_qualified_name.as_deref(),
            Some("::foo::bar"),
            "a call before the deletion must still resolve to the local candidate"
        );
    }

    #[test]
    fn qualified_name_for_var_decl_finds_var_in_matching_top_level_namespace() {
        // TP — the reverse of `lookup_var_in_namespace_finds_var_in_matching_top_level_namespace`
        // (issue #923 idx 68): given the declaration's own span, recover the
        // qualified name an alias elsewhere would name it by.
        let mut root = Scope::new(ScopeKind::Global, "::");
        let mut ns_a = Scope::new(ScopeKind::Namespace, "::A");
        ns_a.variables
            .insert("v".to_string(), var("v", span(10, 11)));
        root.children.push(ns_a);

        assert_eq!(
            qualified_name_for_var_decl(&root, span(10, 11)),
            Some("::A::v".to_string())
        );
    }

    #[test]
    fn local_call_after_deletion_still_falls_back_to_global_issue_973() {
        // FN guard / regression: unlike the case above, `foo::caller` is
        // only ever invoked (at the top level) *after* `rename foo::bar
        // {}` runs, so the local `bar` is genuinely gone by the time the
        // call executes — it must still fall back to the global `::bar`
        // (issue #973's original fix must not regress).
        let mut a = Analyser::new();
        let src = "proc bar {} { return global }\nnamespace eval foo {\n    proc bar {} { return local }\n    rename foo::bar {}\n    proc caller {} { return [bar] }\n}\nfoo::caller\n";
        let r = a.analyse(src, "tcl8.6");
        let bar_call = r
            .command_invocations
            .iter()
            .find(|i| i.name == "bar" && !i.indirect)
            .expect("bar call recorded");
        assert_eq!(
            bar_call.resolved_qualified_name.as_deref(),
            Some("::bar"),
            "a call genuinely after the deletion must fall back to the global candidate"
        );
    }

    #[test]
    fn qualified_name_for_var_decl_accumulates_through_nested_namespaces() {
        // TP — mirrors `lookup_var_in_namespace_accumulates_through_nested_namespaces`.
        let mut root = Scope::new(ScopeKind::Global, "::");
        let mut ns_a = Scope::new(ScopeKind::Namespace, "A");
        let mut ns_b = Scope::new(ScopeKind::Namespace, "B");
        ns_b.variables
            .insert("v".to_string(), var("v", span(20, 21)));
        ns_a.children.push(ns_b);
        root.children.push(ns_a);

        assert_eq!(
            qualified_name_for_var_decl(&root, span(20, 21)),
            Some("::A::B::v".to_string())
        );
    }

    #[test]
    fn escape_hatch_requires_the_specific_enclosing_definitions_own_top_level_call() {
        // FN guard: an unrelated proc's top-level invocation elsewhere in
        // the file must not "lend" liveness to a different enclosing
        // definition that is itself never invoked — the escape hatch must
        // key off the *specific* body's own top-level call, not just
        // "some call happened somewhere before the deletion". `foo::bar`
        // is deleted, and `foo::caller` (the only body that calls it) is
        // never invoked anywhere — only the unrelated `baz::unrelated` is
        // — so the `bar` call must fall back to the global `::bar`.
        let mut a = Analyser::new();
        let src = "proc bar {} { return global }\nnamespace eval foo {\n    proc bar {} { return local }\n    proc caller {} { return [bar] }\n}\nnamespace eval baz {\n    proc unrelated {} { return 1 }\n}\nbaz::unrelated\nrename foo::bar {}\n";
        let r = a.analyse(src, "tcl8.6");
        let bar_call = r
            .command_invocations
            .iter()
            .find(|i| i.name == "bar" && !i.indirect)
            .expect("bar call recorded");
        assert_eq!(
            bar_call.resolved_qualified_name.as_deref(),
            Some("::bar"),
            "foo::caller is never invoked, so the escape hatch must not apply"
        );
    }

    #[test]
    fn escape_hatch_follows_a_chain_of_enclosing_definitions_issue_1015() {
        // FP guard (issue #1015): `foo::caller` is never invoked at the top
        // level — only `foo::entry` is — but `entry` calls `caller`, which
        // calls `bar`, all before the rename, so the local `::foo::bar`
        // stays the resolution. tclsh8.6/9.0 confirm the chain runs clean.
        let mut a = Analyser::new();
        let src = "proc bar {} { return global }\nnamespace eval foo {\n    proc bar {} { return local }\n    proc caller {} { return [bar] }\n    proc entry {} { return [caller] }\n}\nfoo::entry\nrename foo::bar {}\n";
        let r = a.analyse(src, "tcl8.6");
        let bar_call = r
            .command_invocations
            .iter()
            .find(|i| i.name == "bar" && !i.indirect)
            .expect("bar call recorded");
        assert_eq!(
            bar_call.resolved_qualified_name.as_deref(),
            Some("::foo::bar"),
            "foo::entry reaches foo::caller, which reaches bar, before the rename"
        );
    }

    #[test]
    fn escape_hatch_terminates_on_a_never_entered_mutual_recursion_cycle_issue_1015() {
        // TP guard (issue #1015): `foo::ping` and `foo::pong` call each
        // other and nothing calls either, so neither is ever reached — the
        // reachability fixpoint must terminate and leave the escape hatch
        // shut, falling the `bar` call back to the global `::bar`.
        let mut a = Analyser::new();
        let src = "proc bar {} { return global }\nnamespace eval foo {\n    proc bar {} { return local }\n    proc ping {} { pong\n        return [bar] }\n    proc pong {} { return [ping] }\n}\nrename foo::bar {}\n";
        let r = a.analyse(src, "tcl8.6");
        let bar_call = r
            .command_invocations
            .iter()
            .find(|i| i.name == "bar" && !i.indirect)
            .expect("bar call recorded");
        assert_eq!(
            bar_call.resolved_qualified_name.as_deref(),
            Some("::bar"),
            "neither cycle member is ever reached, so the escape hatch must not apply"
        );
    }

    #[test]
    fn qualified_name_for_var_decl_skips_proc_locals() {
        // FP guard — a proc-local variable's declaration span must never
        // produce a qualified name: `ScopeKind::Proc` is excluded from the
        // walk exactly like `lookup_var_in_namespace_skips_proc_locals_even_when_namespace_matches`,
        // since a bare `set v 1` inside a proc has no relation to any
        // namespace cell unless linked via `variable`/`global` (a separate
        // `VarDef` with its own `link_target`, not this one).
        let mut root = Scope::new(ScopeKind::Global, "::");
        let mut ns_a = Scope::new(ScopeKind::Namespace, "::A");
        let mut proc_f = Scope::new(ScopeKind::Proc, "f");
        proc_f
            .variables
            .insert("v".to_string(), var("v", span(40, 41)));
        ns_a.children.push(proc_f);
        root.children.push(ns_a);

        assert!(
            qualified_name_for_var_decl(&root, span(40, 41)).is_none(),
            "a proc-local variable's span must never resolve to a qualified name"
        );
    }

    #[test]
    fn qualified_name_for_var_decl_returns_none_for_unknown_span() {
        // TN — a span that names no declaration anywhere in the tree.
        let mut root = Scope::new(ScopeKind::Global, "::");
        root.variables.insert("v".to_string(), var("v", span(0, 1)));
        assert!(qualified_name_for_var_decl(&root, span(99, 100)).is_none());
    }

    #[test]
    fn qualified_name_for_var_decl_finds_global_scope_variable() {
        // TP — the degenerate root/`::` case: a variable declared directly
        // in the global scope qualifies as `::name`, not `::::name`.
        let mut root = Scope::new(ScopeKind::Global, "::");
        root.variables.insert("v".to_string(), var("v", span(0, 1)));
        assert_eq!(
            qualified_name_for_var_decl(&root, span(0, 1)),
            Some("::v".to_string())
        );
    }

    #[test]
    fn qualified_name_for_var_decl_does_not_double_prefix_a_literal_qualified_name() {
        // TP — issue #923 idx 68: `handle_set_command`/`define_var` never
        // re-qualify a name they're given (`normalise_var_name` only strips
        // a `$`/`${…}` wrapper and an array index), so a literal `set
        // ::tolComp val` stores `VarDef::name == "::tolComp"` verbatim, not
        // the bare tail `"tolComp"`. Prefixing that with `ns` again would
        // produce `"::::tolComp"`, matching no alias's `link_target`.
        let mut root = Scope::new(ScopeKind::Global, "::");
        root.variables
            .insert("::tolComp".to_string(), var("::tolComp", span(50, 60)));
        assert_eq!(
            qualified_name_for_var_decl(&root, span(50, 60)),
            Some("::tolComp".to_string())
        );
    }

    #[test]
    fn lookup_var_by_qualified_name_finds_a_bare_tail_namespace_variable() {
        // TP — the ordinary `variable`-declared namespace cell shape,
        // delegated straight through to `lookup_var_in_namespace`.
        let mut root = Scope::new(ScopeKind::Global, "::");
        let mut ns_a = Scope::new(ScopeKind::Namespace, "::A");
        ns_a.variables
            .insert("v".to_string(), var("v", span(10, 11)));
        root.children.push(ns_a);

        assert_eq!(
            lookup_var_by_qualified_name(&root, "::A::v").map(|v| v.definition_span),
            Some(span(10, 11))
        );
    }

    #[test]
    fn lookup_var_by_qualified_name_finds_a_literal_qualified_top_level_set() {
        // TP — issue #923 idx 68's exact repro shape: a plain `set
        // ::tolComp val` at global scope stores its key verbatim
        // (`"::tolComp"`), which the bare-tail lookup alone (`base_name ==
        // "tolComp"`) can never match; the literal-name fallback must.
        let mut root = Scope::new(ScopeKind::Global, "::");
        root.variables
            .insert("::tolComp".to_string(), var("::tolComp", span(50, 60)));

        assert_eq!(
            lookup_var_by_qualified_name(&root, "::tolComp").map(|v| v.definition_span),
            Some(span(50, 60))
        );
    }

    #[test]
    fn lookup_var_by_qualified_name_finds_an_unqualified_top_level_set() {
        // TP — the other half of idx 68's repro: an *unqualified* `set
        // tolComp val` at global scope stores the bare key `"tolComp"`,
        // found by the existing tail-based lookup with no fallback needed.
        let mut root = Scope::new(ScopeKind::Global, "::");
        root.variables
            .insert("tolComp".to_string(), var("tolComp", span(50, 60)));

        assert_eq!(
            lookup_var_by_qualified_name(&root, "::tolComp").map(|v| v.definition_span),
            Some(span(50, 60))
        );
    }

    #[test]
    fn lookup_var_by_qualified_name_does_not_conflate_unrelated_literal_cells() {
        // FP guard — two literal-qualified cells in different namespaces
        // must never cross-match just because both are reachable via the
        // literal-name fallback.
        let mut root = Scope::new(ScopeKind::Global, "::");
        root.variables
            .insert("::A::x".to_string(), var("::A::x", span(1, 2)));
        root.variables
            .insert("::B::x".to_string(), var("::B::x", span(3, 4)));

        assert_eq!(
            lookup_var_by_qualified_name(&root, "::A::x").map(|v| v.definition_span),
            Some(span(1, 2))
        );
        assert_eq!(
            lookup_var_by_qualified_name(&root, "::B::x").map(|v| v.definition_span),
            Some(span(3, 4))
        );
    }

    #[test]
    fn lookup_var_by_qualified_name_returns_none_for_unknown_target() {
        // TN
        let root = Scope::new(ScopeKind::Global, "::");
        assert!(lookup_var_by_qualified_name(&root, "::Nope::v").is_none());
    }
}
