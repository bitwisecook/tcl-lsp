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

//! The namespace an unqualified command resolves in at a source offset, the implicit `namespace path` and the `TclOO` frame predicates.

use super::super::state::Analyser;
use super::super::types::{Scope, ScopeKind};
use tcl_lexer::Span;

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
pub(super) fn advance_command_resolution_namespace(ns: &str, child: &Scope) -> String {
    if let Some(display) = child
        .naming_scope
        .as_ref()
        .and_then(crate::signature_scan::scope::SignatureNamespaceScope::display)
    {
        if let Some(domain) = &child.original_interpreter_source_domain {
            return child
                .naming_scope
                .as_ref()
                .and_then(|namespace| domain.reported_namespace(namespace))
                .unwrap_or_else(|| ns.to_owned());
        }
        return display;
    }
    match child.kind {
        ScopeKind::Namespace => crate::naming::qualify_namespace(ns, &child.name),
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
            // prefix (`proc :` at the root defines in `::`, never in a
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

/// Select only the actual innermost method body for an original-owner rejoin.
/// This span is not an entry receipt: the caller must match the full document,
/// input and genuine current wrapper before issuing any first-entry advice.
pub(crate) fn selected_jim_method_body_span_at(root: &Scope, offset: u32) -> Option<Span> {
    let mut scope = root;
    while let Some(child) = innermost_containing_child(scope, offset) {
        scope = child;
    }
    (scope.kind == ScopeKind::Method).then_some(())?;
    scope.body_span
}

/// The innermost child scope of `cursor` whose `body_span` contains
/// `byte_offset` — the one step both byte-offset scope walks below take.
///
/// "Innermost" is by **span width**, not by first match: the scope tree is
/// not strictly nested in one case that matters here. An `apply` lambda
/// written inside a `TclOO` method body opens its `ScopeKind::Proc` scope
/// as a *sibling* of the method's own scope (both hang off the enclosing
/// scope, since the method body is walked in its own pass) while its span
/// lies strictly inside the method's. Taking the first container would stop
/// at the method and miss the lambda — which is exactly the frame that
/// matters, because `apply` runs its body in the global namespace and so
/// loses the object context (tclsh 9.0.4: `apply {{} { link Helper }}`
/// inside a method raises `invalid command name "link"`).
/// Where the tree *is* properly nested, no two children of one node
/// contain the same offset, so this is the plain first-match walk.
pub(super) fn innermost_containing_child(cursor: &Scope, byte_offset: u32) -> Option<&Scope> {
    cursor
        .children
        .iter()
        .filter_map(|c| {
            c.body_span
                .filter(|s| s.start() <= byte_offset && byte_offset < s.end())
                .map(|s| (c, s.end() - s.start()))
        })
        .min_by_key(|&(_, width)| width)
        .map(|(c, _)| c)
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
    while let Some(child) = innermost_containing_child(cursor, byte_offset) {
        ns = advance_command_resolution_namespace(&ns, child);
        cursor = child;
    }
    ns
}

/// Retained source naming geometry at the actual innermost scope. A scope
/// without its own original geometry withdraws the query instead of donating
/// a parent or presentation-derived namespace to its body.
#[must_use]
pub fn original_namespace_scope_at(
    root: &Scope,
    byte_offset: u32,
) -> Option<&crate::signature_scan::scope::SignatureNamespaceScope> {
    let mut cursor = root;
    while let Some(child) = innermost_containing_child(cursor, byte_offset) {
        cursor = child;
    }
    cursor.naming_scope.as_ref()
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
/// alongside the ordinary `call_ns == target_ns` case.
///
/// Same traversal as [`command_resolution_namespace_at`] (so the two can
/// never disagree about which scope is innermost), tracking whether the
/// *last* namespace-resolution-affecting scope kind visited on the way
/// down was an `oo_global_resolution` method — a `namespace eval` (or any
/// other scope kind) nested inside a method body leaves the method's
/// object-context resolution behind, exactly like [`ScopeKind::Namespace`]
/// resets nothing in `advance_command_resolution_namespace` but a
/// `Proc`/`Method`/`Uplevel` scope does.
///
/// Because a `TclOO` object frame is exactly where `::oo::Helpers` sits on
/// the path, this is also the **resolution** half of the `oo::Helpers`
/// scoping rule — including the per-object `my`, which is not
/// an `::oo::Helpers` member at all (`namespace which -command my` answers
/// `::oo::ObjN::my` under tclsh 9.0.4) yet is reachable in exactly the same
/// frames and nowhere else. The `Proc` reset is what makes an `apply` lambda
/// written *inside* a method correctly not count: tclsh 9.0.4 raises `invalid
/// command name "link"` / `"my"` / `"self"` there.
///
/// **"Resolves here" and "is callable here" are different facts, and this
/// answers only the first.** `W123` — "is this an
/// unknown command" — keys on resolution and therefore on this predicate.
/// Completion and hover ask "may the user write this word here", which is
/// callability, and must use [`innermost_scope_is_oo_method_frame`] instead:
/// a Tcl 9 class `initialise` body sets `oo_global_resolution` (so this
/// answers `true`, correctly suppressing `W123`) while every family member
/// but `my` raises `… may only be called from inside a method` when actually
/// called there.
#[must_use]
pub fn innermost_scope_reaches_oo_helpers(root: &Scope, byte_offset: u32) -> bool {
    innermost_oo_frame(root, byte_offset, |child| child.oo_global_resolution)
}

/// The **implicit `namespace path`** a bare command word written at
/// `byte_offset` resolves through — the namespaces Tcl searches after the
/// frame's own current namespace and before global, without anyone having
/// written a `namespace path` command.
///
/// Empty everywhere except a `TclOO` object frame, where the path is
/// registry data
/// ([`tcl_registry::definer::TCLOO_MEMBER_BODY_NAMESPACE_PATH`]) rather than
/// a namespace name spelled in this crate.  [`Scope::oo_global_resolution`]
/// — the flag [`innermost_scope_reaches_oo_helpers`] reads — is precisely
/// "this frame resolves like a `TclOO` member body", so the two facts are
/// one lookup and cannot drift apart.
///
/// This is the single recording site for the path: the
/// analyser folds it into every affected invocation's
/// `resolution_candidates`, so a cross-document consumer walking that list
/// finds a `proc ::oo::Helpers::NAME` declared in *another* file without
/// growing its own lenient fallback.
#[must_use]
pub fn implicit_command_namespace_path_at(
    root: &Scope,
    byte_offset: u32,
) -> &'static [&'static str] {
    if innermost_scope_reaches_oo_helpers(root, byte_offset) {
        tcl_registry::definer::TCLOO_MEMBER_BODY_NAMESPACE_PATH
    } else {
        &[]
    }
}

/// Whether the innermost scope containing `byte_offset` is a real `TclOO`
/// **method invocation** — the *callability* half of the `oo::Helpers`
/// scoping rule, and the counterpart to
/// [`innermost_scope_reaches_oo_helpers`]'s *resolution* half.
///
/// Identical traversal, reading [`Scope::oo_method_frame`] instead of
/// [`Scope::oo_global_resolution`], so the two can never disagree about
/// which scope is innermost. They differ in exactly one place: a Tcl 9
/// class-level `initialise` / `initialize` body. Pinned against tclsh
/// 9.0.4, inside `oo::class create ::P { initialize { … } }`:
///
/// ```text
/// ns=::oo::Obj20  path=::oo::Helpers ::oo
/// link:  which='::oo::Helpers::link'  call -> link may only be called from inside a method
/// self:  which='::oo::Helpers::self'  call -> self may only be called from inside a method
/// my:    which='::oo::Obj20::my'      call -> OK  (`my new` returns ::oo::Obj22)
/// ```
///
/// Consumers pair this with
/// [`tcl_registry::CommandRegistry::requires_oo_method_frame`], which says
/// *which* words need the stronger frame — the five `::oo::Helpers` members
/// do, `my` does not, because it is the object's own dispatch command and a
/// class is an object.
#[must_use]
pub fn innermost_scope_is_oo_method_frame(root: &Scope, byte_offset: u32) -> bool {
    innermost_oo_frame(root, byte_offset, |child| child.oo_method_frame)
}

/// Shared descent for the two `TclOO` frame predicates above: walk to the
/// innermost containing scope, remembering what the *last*
/// namespace-resolution-affecting scope kind on the way down said.
///
/// A `namespace eval` (or any other scope kind) nested inside a method body
/// leaves the method's object-context resolution behind, exactly like
/// [`ScopeKind::Namespace`] resets nothing in
/// [`advance_command_resolution_namespace`] but a `Proc` / `Method` /
/// `Uplevel` scope does.
fn innermost_oo_frame(root: &Scope, byte_offset: u32, method_says: fn(&Scope) -> bool) -> bool {
    let mut reaches = false;
    let mut cursor = root;
    while let Some(child) = innermost_containing_child(cursor, byte_offset) {
        reaches = match child.kind {
            ScopeKind::Namespace | ScopeKind::Global => reaches,
            ScopeKind::Method => method_says(child),
            ScopeKind::Proc | ScopeKind::Uplevel => false,
        };
        cursor = child;
    }
    reaches
}

impl Analyser {
    /// Namespace in which an *unqualified* command invoked at
    /// `scope_path` resolves, following Tcl's command-resolution rule:
    /// the call's enclosing namespace, where a proc body resolves
    /// commands in the proc's **defining** namespace (the prefix of its
    /// qualified name) rather than its lexical parent.
    ///
    /// This is deliberately **not** a purely lexical namespace walk.
    /// `proc ::ns::p {...}` declared at top level has defining namespace
    /// `::ns`, so an unqualified `foo` in its body resolves to `::ns::foo`
    /// then `::foo` — even though there is no enclosing `namespace eval`.
    /// A lexical walk (one that skips proc scopes) answers `::` there and
    /// mis-homes everything the body creates or looks up: procs, classes,
    /// ensembles, `namespace import`/`export` targets, and aliases.
    /// Every analyser site that needs "the namespace
    /// current at this point" uses this one rule.
    #[must_use]
    pub(in super::super) fn command_resolution_namespace(&self, scope_path: &[usize]) -> String {
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
    pub(in super::super) fn scope_path_in_proc_body(&self, scope_path: &[usize]) -> bool {
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
    pub(in super::super) fn scope_path_in_method_body(&self, scope_path: &[usize]) -> bool {
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

    /// Retained declaration context of the innermost method body. A nested
    /// procedure or lambda owns another frame and resets this readonly context.
    #[must_use]
    pub(in super::super) fn current_method_context(
        &self,
        scope_path: &[usize],
    ) -> Option<&super::super::types::OriginalLexicalMemberContext> {
        let mut cursor = &self.result.global_scope;
        let mut found = None;
        for &index in scope_path {
            let child = cursor.children.get(index)?;
            match child.kind {
                ScopeKind::Method => found = child.original_member_context.as_deref(),
                ScopeKind::Proc => found = None,
                ScopeKind::Global | ScopeKind::Namespace | ScopeKind::Uplevel => {}
            }
            cursor = child;
        }
        found
    }
}
