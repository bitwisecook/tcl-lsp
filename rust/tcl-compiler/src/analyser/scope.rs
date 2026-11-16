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
//! tracking maps (``const_strings``, ``regex_vars``,
//! ``result.regex_patterns``).

use std::collections::{HashMap, HashSet};

use tcl_core_types::DiagCode;
use tcl_lexer::{Span, Token, TokenType};

use super::state::Analyser;
use super::types::{ClassDef, ProcDef, Scope, ScopeKind, VarDef};

mod logical_array_indices;
pub use logical_array_indices::{LogicalArrayIndexCandidate, LogicalArrayIndexKind};

pub(super) struct VariableLinkTarget {
    pub(super) target: String,
    pub(super) target_span: Span,
    pub(super) local_span: Span,
    pub(super) original_target_namespace:
        Option<crate::signature_scan::scope::SignatureNamespaceScope>,
    pub(super) original_target_receiver:
        crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver,
}

fn original_dictionary_key_spans(
    value: &str,
    original: Option<Span>,
) -> HashMap<String, Option<Span>> {
    let mut original_keys = std::collections::HashMap::new();
    if let Some(content) = original {
        let mut offset = 0;
        let mut ordinal = 0;
        while let Ok(Some(element)) = tcl_syntax::list::find_element(value, offset) {
            if ordinal % 2 == 0 && element.literal {
                let key = &value[element.value.clone()];
                let span = u32::try_from(element.value.start)
                    .ok()
                    .zip(u32::try_from(element.value.end).ok())
                    .and_then(|(start, end)| {
                        Some(Span::new(
                            content.start().checked_add(start)?,
                            content.start().checked_add(end)?,
                        ))
                    });
                original_keys
                    .entry(key.to_owned())
                    .and_modify(|entry| *entry = None)
                    .or_insert(span);
            }
            ordinal += 1;
            offset = element.next;
        }
    }
    original_keys
}

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
pub(super) fn scope_at<'a>(root: &'a Scope, path: &[usize]) -> Option<&'a Scope> {
    let mut cursor = root;
    for &idx in path {
        cursor = cursor.children.get(idx)?;
    }
    Some(cursor)
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
fn innermost_containing_child(cursor: &Scope, byte_offset: u32) -> Option<&Scope> {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogicalVariableScopeKind {
    Root,
    Namespace,
    UnknownNamespace,
    Local,
    UnknownCaller,
}

/// Eligible variable declaration scopes from the current Logical source.
/// Cell identities, entered links and editable symbols require their own owners.
#[derive(Debug)]
pub struct LogicalVariableScopeAdvice<'a> {
    analysis: &'a super::types::AnalysisResult,
    scopes: Vec<&'a Scope>,
    eligible: Vec<usize>,
    kind: LogicalVariableScopeKind,
    namespace: Option<String>,
    image: tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
}

/// A dictionary key inferred from an original checked value, without a variable declaration span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadonlyDictionaryKeyVariable {
    name: String,
    image: tcl_lexer::SourceImage,
    input: super::ResolvedAnalysisInput,
    receiver_scope: Vec<usize>,
    original_key: Option<Span>,
    original_declaration: Option<Span>,
}

impl ReadonlyDictionaryKeyVariable {
    /// Source-value label, independently of entered variable storage.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Exact original data-key interior, when unchanged literal correspondence exists.
    /// This is source geometry and supplies no variable rename permission.
    #[must_use]
    pub const fn original_key_span(&self) -> Option<Span> {
        self.original_key
    }
}

impl<'a> LogicalVariableScopeAdvice<'a> {
    /// Root declaration table, independently of its lookup eligibility.
    #[must_use]
    pub fn root(&self) -> &'a Scope {
        &self.analysis.global_scope
    }

    /// Eligible source tables in outer-to-inner order.
    pub fn eligible_scopes(&self) -> impl DoubleEndedIterator<Item = &'a Scope> + '_ {
        self.eligible.iter().map(|&index| self.scopes[index])
    }

    /// Fully rooted candidates from other authentic namespace declaration tables.
    /// Each original declaration supplies its own retained namespace coordinate;
    /// reporting scope names cannot construct insertion text or establish a cell.
    #[must_use]
    pub fn cross_namespace_variable_names(&self) -> Vec<String> {
        let mut pending = vec![self.root()];
        let mut names = Vec::new();
        while let Some(scope) = pending.pop() {
            pending.extend(&scope.children);
            if !matches!(scope.kind, ScopeKind::Global | ScopeKind::Namespace)
                || self
                    .eligible_scopes()
                    .any(|current| std::ptr::eq(current, scope))
            {
                continue;
            }
            for (name, declaration) in &scope.variables {
                let Some(original) = self.analysis.logical_variable_scope_advice(
                    &self.image,
                    self.config,
                    declaration.definition_span.start(),
                ) else {
                    continue;
                };
                if !matches!(
                    original.kind,
                    LogicalVariableScopeKind::Root | LogicalVariableScopeKind::Namespace
                ) || !original
                    .eligible_scopes()
                    .any(|current| std::ptr::eq(current, scope))
                {
                    continue;
                }
                let Some(namespace) = original.namespace() else {
                    continue;
                };
                let coordinate = if crate::naming::unroot_rooted_key(name).is_some() {
                    name.clone()
                } else {
                    tcl_syntax::naming::rooted_member_key(namespace, name)
                };
                if self
                    .variable_definition(&coordinate)
                    .is_some_and(|selected| std::ptr::eq(selected, declaration))
                {
                    names.push(coordinate);
                }
            }
        }
        names.sort_unstable();
        names.dedup();
        names
    }

    /// Inferred source names from the eligible Logical tables, without editable declarations.
    #[must_use]
    pub fn readonly_variable_names(&self) -> Vec<&'a str> {
        self.eligible_scopes()
            .flat_map(|scope| {
                scope
                    .readonly_dictionary_keys
                    .values()
                    .filter(move |key| {
                        key.image == self.image
                            && self.analysis.resolved_input.as_ref() == Some(&key.input)
                            && scope_at(&self.analysis.global_scope, &key.receiver_scope)
                                .is_some_and(|retained| std::ptr::eq(retained, scope))
                    })
                    .map(ReadonlyDictionaryKeyVariable::name)
            })
            .collect()
    }

    /// Inferred names held by the independent root table under current source currency.
    #[must_use]
    pub fn readonly_root_variable_names(&self) -> Vec<&'a str> {
        self.root()
            .readonly_dictionary_keys
            .values()
            .filter(|key| {
                key.image == self.image
                    && self.analysis.resolved_input.as_ref() == Some(&key.input)
                    && key.receiver_scope.is_empty()
            })
            .map(ReadonlyDictionaryKeyVariable::name)
            .collect()
    }

    /// Known source namespace; an unknown caller cannot supply one.
    #[must_use]
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    /// A declaration table selected by this source frame's naming policy.
    /// The result establishes no entered storage cell or editable identity.
    #[must_use]
    pub fn variable_definition(&self, name: &str) -> Option<&'a VarDef> {
        if crate::naming::is_qualified(name.as_bytes())
            && let Some(qualified) = self.namespace_variable_coordinate(name)
        {
            let (holder, tail) = crate::naming::key_holder_and_tail(&qualified);
            return self.namespace_variable_definition(holder, tail);
        }
        self.variable_declaration_scope(name)?.variables.get(name)
    }

    fn namespace_variable_definition(&self, holder: &str, tail: &str) -> Option<&'a VarDef> {
        fn collect<'a>(
            advice: &LogicalVariableScopeAdvice<'a>,
            scope: &'a Scope,
            holder: &str,
            tail: &str,
            found: &mut Vec<&'a VarDef>,
        ) {
            if matches!(scope.kind, ScopeKind::Global | ScopeKind::Namespace)
                && let Some(definition) = scope.variables.get(tail)
                && let Some(frame) = advice.analysis.logical_variable_scope_advice(
                    &advice.image,
                    advice.config,
                    definition.definition_span.start(),
                )
                && matches!(
                    frame.kind,
                    LogicalVariableScopeKind::Root | LogicalVariableScopeKind::Namespace
                )
                && frame.namespace() == Some(holder)
                && frame
                    .eligible_scopes()
                    .any(|eligible| std::ptr::eq(eligible, scope))
            {
                found.push(definition);
            }
            for child in &scope.children {
                collect(advice, child, holder, tail, found);
            }
        }
        let mut found = Vec::new();
        collect(self, self.root(), holder, tail, &mut found);
        let [definition] = found.as_slice() else {
            return None;
        };
        Some(*definition)
    }

    fn readonly_key_is_current(&self, scope: &Scope, name: &str) -> bool {
        scope.readonly_dictionary_keys.get(name).is_some_and(|key| {
            key.image == self.image
                && self.analysis.resolved_input.as_ref() == Some(&key.input)
                && scope_at(&self.analysis.global_scope, &key.receiver_scope)
                    .is_some_and(|retained| std::ptr::eq(retained, scope))
        })
    }

    fn variable_declaration_scope(&self, name: &str) -> Option<&'a Scope> {
        let current = || {
            self.eligible_scopes().rev().find(|scope| {
                scope.variables.contains_key(name)
                    || (self.readonly_key_is_current(scope, name)
                        && scope.readonly_dictionary_keys[name]
                            .original_declaration
                            .is_none())
            })
        };
        if !crate::naming::is_qualified(name.as_bytes()) && self.is_namespace_frame() {
            return crate::naming::variable_namespace_lookup_bases(
                false,
                self,
                self.namespace_global_fallback(),
            )
            .find_map(|base| match base {
                crate::naming::VariableLookupBase::Context(_) => current(),
                crate::naming::VariableLookupBase::Root => self
                    .root()
                    .variables
                    .contains_key(name)
                    .then(|| self.root()),
            });
        }
        current()
    }

    /// Namespace variable coordinate from this checked Logical source frame.
    /// Local names and relative names in unknown callers supply no coordinate.
    #[must_use]
    pub fn namespace_variable_coordinate(&self, written: &str) -> Option<String> {
        let namespace = if crate::naming::unroot_rooted_key(written).is_some() {
            "::"
        } else {
            if !crate::naming::is_qualified(written.as_bytes())
                && !matches!(
                    self.kind,
                    LogicalVariableScopeKind::Root | LogicalVariableScopeKind::Namespace
                )
            {
                return None;
            }
            self.namespace()?
        };
        Some(crate::naming::qualify(namespace, written))
    }

    /// Namespace tables use their selected root alternative independently.
    #[must_use]
    pub fn is_namespace_frame(&self) -> bool {
        self.kind == LogicalVariableScopeKind::Namespace
    }

    /// Root names require explicit rooted insertion outside a root frame.
    #[must_use]
    pub fn root_names_require_qualification(&self) -> bool {
        match self.kind {
            LogicalVariableScopeKind::Root => false,
            LogicalVariableScopeKind::Namespace => !self.analysis.ns_var_global_fallback(),
            LogicalVariableScopeKind::UnknownNamespace
            | LogicalVariableScopeKind::Local
            | LogicalVariableScopeKind::UnknownCaller => true,
        }
    }

    /// Fallback policy from the same independently retained Logical input.
    #[must_use]
    pub fn namespace_global_fallback(&self) -> bool {
        self.analysis.ns_var_global_fallback()
    }
}

impl super::types::AnalysisResult {
    /// A complete retained Logical word reproduces this readonly input.
    /// Native words, computed values and unretained source vectors cannot use this correspondence.
    #[must_use]
    pub fn logical_original_word_input_matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        span: Span,
        input: &crate::signature_scan::scope::SignatureSourceNameInput,
    ) -> bool {
        use crate::signature_scan::scope::SignatureSourceNameInput;
        let SignatureSourceNameInput::OriginalWord(retained) = input else {
            return false;
        };
        if !self.allows_lexical_declaration_advice()
            || !self.matches_original_source_image(image, config)
        {
            return false;
        }
        let Some(resolved) = self.resolved_input.as_ref() else {
            return false;
        };
        if !resolved.has_logical_source_name_context() {
            return false;
        }
        let Some(policy) = tcl_registry::InvocationDialect::of_profile(resolved.analyser_profile())
            .authored_name_policy()
        else {
            return false;
        };
        if input.policy() != policy {
            return false;
        }
        let Some((word, _)) = self
            .retained_command_realm()
            .and_then(|realm| realm.original_written_word_at_span_in_source(image, span, config))
        else {
            return false;
        };
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&config);
        crate::signature_scan::scope::SignatureSourceNameKey::from_original_logical_source_word(
            &word, rules, policy,
        )
        .as_ref()
            == Some(retained)
    }

    /// Current source scope eligibility for independently retained Logical advice.
    /// Original Native/hosted bodies and readonly alias receipts cannot issue it.
    #[must_use]
    pub fn logical_variable_scope_advice(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        offset: u32,
    ) -> Option<LogicalVariableScopeAdvice<'_>> {
        if !self.allows_lexical_declaration_advice()
            || !self.matches_original_source_image(image, config)
            || usize::try_from(offset).ok()? > image.bytes().len()
        {
            return None;
        }
        let registry = self.resolved_registry()?;
        let mut scopes = vec![&self.global_scope];
        let mut cursor = &self.global_scope;
        while let Some(child) = innermost_containing_child(cursor, offset) {
            scopes.push(child);
            cursor = child;
        }
        for scope in &scopes {
            if scope.is_independent_interpreter_source_body() {
                return None;
            }
            match (
                &self.global_scope.original_interpreter_source_domain,
                &scope.original_interpreter_source_domain,
            ) {
                (None, None) => {}
                (Some(root), Some(current)) if std::sync::Arc::ptr_eq(root, current) => {}
                _ => return None,
            }
        }
        let mut eligible = vec![0];
        let mut kind = LogicalVariableScopeKind::Root;
        let mut namespace = Some("::".to_owned());
        for (index, scope) in scopes.iter().enumerate().skip(1) {
            match scope.kind {
                ScopeKind::Global => eligible.push(index),
                ScopeKind::Namespace => {
                    eligible.clear();
                    eligible.push(index);
                    kind = LogicalVariableScopeKind::Namespace;
                    namespace = if namespace.is_none()
                        && tcl_syntax::naming::unroot_rooted_key(&scope.name).is_none()
                    {
                        None
                    } else {
                        scope.naming_scope.as_ref().and_then(
                            crate::signature_scan::scope::SignatureNamespaceScope::display,
                        )
                    };
                    if namespace.is_none() {
                        kind = LogicalVariableScopeKind::UnknownNamespace;
                    }
                    if namespace
                        .as_deref()
                        .is_some_and(|name| tcl_syntax::naming::unroot_rooted_key(name) == Some(""))
                    {
                        eligible.insert(0, 0);
                        kind = LogicalVariableScopeKind::Root;
                    }
                }
                ScopeKind::Proc | ScopeKind::Method => {
                    eligible.clear();
                    eligible.push(index);
                    kind = LogicalVariableScopeKind::Local;
                    namespace = if namespace.is_none()
                        && tcl_syntax::naming::unroot_rooted_key(&scope.name).is_none()
                    {
                        None
                    } else {
                        scope.naming_scope.as_ref().and_then(
                            crate::signature_scan::scope::SignatureNamespaceScope::display,
                        )
                    };
                }
                ScopeKind::Uplevel => {
                    let level = tcl_registry::FrameLevel::parse_in(&scope.name, registry);
                    if level.is_some_and(tcl_registry::FrameLevel::is_current_frame) {
                        eligible.push(index);
                    } else if level.is_some_and(tcl_registry::FrameLevel::is_global_frame) {
                        eligible.clear();
                        eligible.extend([0, index]);
                        kind = LogicalVariableScopeKind::Root;
                        namespace = Some("::".to_owned());
                    } else {
                        eligible.clear();
                        eligible.push(index);
                        kind = LogicalVariableScopeKind::UnknownCaller;
                        namespace = None;
                    }
                }
            }
        }
        Some(LogicalVariableScopeAdvice {
            analysis: self,
            scopes,
            eligible,
            kind,
            namespace,
            image: image.clone(),
            config,
        })
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

/// One recursive step of [`Analyser::attach_qualified_var_references`]:
/// append the occurrence spans `by_cell` holds for each namespace cell in
/// `node`'s subtree to that cell's [`VarDef`].  `ns` is `node`'s accumulated
/// resolution namespace; `by_cell` is keyed by `::`-stripped cell name.
fn attach_qualified_var_refs_to_scope(
    ns: &str,
    node: &mut Scope,
    by_cell: &HashMap<&str, Vec<Span>>,
) {
    if matches!(node.kind, ScopeKind::Namespace | ScopeKind::Global) {
        for var in node.variables.values_mut() {
            let qualified = crate::naming::qualify(ns, &var.name);
            let Some(spans) = by_cell
                .get(tcl_syntax::naming::unroot_rooted_key(&qualified).unwrap_or(&qualified))
            else {
                continue;
            };
            for &span in spans {
                if span != var.definition_span && !var.references.contains(&span) {
                    var.references.push(span);
                }
            }
            var.references.sort_by_key(|s| (s.start(), s.end()));
        }
    }
    let child_namespaces: Vec<String> = node
        .children
        .iter()
        .map(|child| advance_command_resolution_namespace(ns, child))
        .collect();
    for (child, child_ns) in node.children.iter_mut().zip(child_namespaces) {
        attach_qualified_var_refs_to_scope(&child_ns, child, by_cell);
    }
}

/// Every namespace-scoped variable declaration in `root`, each paired with the
/// `::`-rooted qualified name it is reachable under.
///
/// The enumerating twin of [`lookup_var_in_namespace`] — same traversal,
/// same rule about which frames count ([`ScopeKind::Namespace`] /
/// [`ScopeKind::Global`] only, never a proc's or method's locals), so a
/// consumer that indexes these and one that looks a single name up can
/// never disagree about what a namespace's variable table holds.
///
/// Used by the cross-document workspace index: a namespace variable is the
/// one kind of variable that has a name every document can spell, so it is
/// the one kind worth lifting out of a single document's scope tree.
#[must_use]
pub fn namespace_variables(root: &Scope) -> Vec<(String, &VarDef)> {
    fn walk<'a>(ns: &str, node: &'a Scope, out: &mut Vec<(String, &'a VarDef)>) {
        if matches!(node.kind, ScopeKind::Namespace | ScopeKind::Global) {
            for var in node.variables.values() {
                out.push((crate::naming::qualify(ns, &var.name), var));
            }
        }
        for child in &node.children {
            let child_ns = advance_command_resolution_namespace(ns, child);
            walk(&child_ns, child, out);
        }
    }
    let mut out = Vec::new();
    walk("::", root, &mut out);
    // `Scope::variables` is a `HashMap`, so the walk's per-node order is the
    // process hash seed's; sort by declaration span (then name) for a stable,
    // meaningful index order — the same reasoning as `sorted_by_span` on the
    // proc/class side.
    out.sort_by(|(qa, a), (qb, b)| {
        a.definition_span
            .start()
            .cmp(&b.definition_span.start())
            .then(a.definition_span.end().cmp(&b.definition_span.end()))
            .then_with(|| qa.cmp(qb))
    });
    out
}

/// One local **alias** of a namespace/global cell: the cell it names, and the
/// [`VarDef`] holding the alias.
///
/// See [`variable_alias_links`].
#[derive(Debug, Clone, Copy)]
pub struct VariableAliasLink<'a> {
    /// The `::`-rooted cell the alias binds to (`VarDef::link_target`).
    pub cell: &'a str,
    /// The alias record itself — its [`VarDef::link_target_span`] is the word
    /// a rename of `cell` must rewrite, and its own `definition_span` /
    /// `references` are the local spelling.
    pub var: &'a VarDef,
}

/// Every variable in `root` that **aliases** a namespace/global cell — a
/// `variable v`, `global v`, `namespace upvar ns v local`, or `upvar #0 ::ns::v
/// local` — paired with the cell it binds.
///
/// The alias counterpart of [`namespace_variables`], and the second half of
/// the same rule: `namespace_variables` enumerates the cells a document
/// *declares*, this enumerates the ones it *binds to*.  Unlike declarations,
/// an alias can live in any scope and in any namespace — a global `proc p {}
/// { namespace upvar ::ns v local; … }` binds `::ns::v` from `::`, declaring
/// nothing in `::ns` and writing no qualified occurrence — so both consumers
/// need this walk to answer completely:
///
/// * the workspace index, so a rename of `::ns::v` visits every document that
///   binds the cell rather than only those declaring or spelling it
///   (`WorkspaceIndex::documents_aliasing_variable`);
/// * the rename itself, so the word naming the cell is the word rewritten.
///
/// Every scope kind is walked, precisely because a proc's locals are where
/// aliases live.
#[must_use]
pub fn variable_alias_links(root: &Scope) -> Vec<VariableAliasLink<'_>> {
    fn walk<'a>(node: &'a Scope, out: &mut Vec<VariableAliasLink<'a>>) {
        for var in node.variables.values() {
            if let Some(cell) = var.link_target.as_deref() {
                out.push(VariableAliasLink { cell, var });
            }
        }
        for child in &node.children {
            walk(child, out);
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    // `Scope::variables` is a `HashMap`, so the per-node order is the process
    // hash seed's; sort by declaration span (then cell) for the same reason
    // `namespace_variables` does — a stable, source-ordered index.
    out.sort_by(|a, b| {
        a.var
            .definition_span
            .start()
            .cmp(&b.var.definition_span.start())
            .then(
                a.var
                    .definition_span
                    .end()
                    .cmp(&b.var.definition_span.end()),
            )
            .then_with(|| a.cell.cmp(b.cell))
    });
    out
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
    /// *after* that offset (a proc/class/rename/alias target
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
    /// [`crate::command_binding::SourceInvocationBinding::selected_slot_presence`] is (the W123 pass's answer to the
    /// identical question) — a namespaced local candidate must not lose to
    /// the global one just because *some later* deletion exists, when this
    /// specific call runs before it: `proc bar
    /// {}`, `namespace eval foo { proc bar {}; proc caller {} { bar } }`,
    /// `foo::caller`, `rename foo::bar {}` still resolves `bar` (called
    /// from `foo::caller`, before the rename) to `::foo::bar`, not the
    /// global `::bar` — confirmed against tclsh 8.6.14 — because
    /// `foo::caller`'s own top-level invocation already ran before the
    /// rename.
    ///
    /// The enclosing definition need not be called at the top level
    /// *itself* — [`Analyser::reachable_call_offsets`] answers the
    /// transitive question, so an arbitrarily deep chain of
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
        self.user_definition_known(qualified, call_off)
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
            || (self
                .builtins
                .contains(tcl_syntax::naming::unroot_rooted_key(qualified).unwrap_or(qualified))
                && !self.renamed_away.contains(qualified))
    }

    /// Whether this candidate is specifically a proc/class definition which
    /// is live for this call.  Kept separate from [`Self::known`] so indexed
    /// reference settlement can preserve a position-sensitive definition
    /// target without mistaking a builtin, alias, or rename target for that
    /// historical definition.
    fn user_definition_known(&self, qualified: &str, call_off: u32) -> bool {
        self.procs
            .get(qualified)
            .is_some_and(|p| self.live_for_call(qualified, p.name_span.start(), call_off))
            || self
                .classes
                .get(qualified)
                .is_some_and(|c| self.live_for_call(qualified, c.name_span.start(), call_off))
    }
}

/// Record the candidate which the local, position-sensitive command table
/// selected, including whether it is a navigable proc/class definition.
fn record_known_winner<A>(
    inv: &mut crate::signature_scan::types::SignatureCommandInvocation,
    resolved: &str,
    winner: &str,
    known_ctx: &KnownPredicateCtx<'_, A>,
    call_off: u32,
) {
    inv.resolved_user_definition = known_ctx.user_definition_known(winner, call_off);
    if winner != resolved {
        inv.resolved_qualified_name = Some(winner.to_owned());
    }
}

/// The [`VarDef`] a fully qualified `target` (always `::`-rooted; see
/// [`crate::analyser::handlers::Analyser::apply_state_transitions`]'s
/// convention for a `global` alias) names, when it was declared by a *literal* `set` whose own
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
/// general case; only the realistic top-level / namespace-body shape is
/// needed here). Matches by exact
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
/// needs to fold a `global`/`variable`/`namespace upvar`
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
/// scope names via its own `link_target` (a top-level `set tolComp` /
/// `set ::tolComp`, aliased inside a
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
            return Some(if tcl_syntax::naming::is_absolute(&v.name) {
                v.name.clone()
            } else {
                tcl_syntax::naming::rooted_member_key(ns, &v.name)
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
    /// `VarDef.references` would carry only the registry `VarRead`-role
    /// name words (`set x`, `info exists x`) collected at its end.
    ///
    /// The shared configured reference scanner verifies the
    /// original token extent and owns the full form, including literal
    /// brace-quoted names and separate array-index syntax.
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
        // Each read carries whether it used the `${…}` brace form, whose
        // content is a *literal* name: `${$n}` reads the variable called `$n`,
        // not `n`.
        let mut reads: Vec<(String, tcl_lexer::Span, bool)> = Vec::new();
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
            let Some((name, braced)) = crate::var_refs::original_variable_reference_form(
                &self.source,
                *tok,
                self.file_lexer_config(),
            ) else {
                continue;
            };
            reads.push((name.to_string(), tok.span, braced));
        }
        for (name, span, braced) in reads {
            self.record_var_read_braced_in_origin(&name, span, scope_path, braced, true);
        }

        // `$var` reads also occur inside command substitutions
        // (`[…]`) and braced `expr` / condition arguments, which the
        // main body walk treats as opaque tokens.  Collect those
        // here too so `VarDef.references` is complete for the
        // references / document-highlight providers and the
        // minifier's rename pass.  `record_var_read` no-ops on names
        // that aren't in the current scope, so liberal collection is
        // safe.
        if let Some(registry) = self.registry.as_deref() {
            let mut extra: Vec<VarReadSite> = Vec::new();
            let lexer_config = self.file_lexer_config();
            let expr_idx: std::collections::HashSet<usize> =
                crate::registry_invocation::source_structure::source_registry_words(
                    &self.source,
                    &self.result,
                    cmd,
                )
                .map_or_default(|selected| selected.written_argument_roles())
                .into_iter()
                .filter_map(|(index, role)| (role == tcl_registry::ArgRole::Expr).then_some(index))
                .collect();
            // Top-level command-substitution tokens.
            for tok in &cmd.all_tokens {
                if tok.kind == tcl_lexer::TokenType::Cmd {
                    collect_cmd_subst_reads(
                        &self.source,
                        &self.result,
                        *tok,
                        registry,
                        lexer_config,
                        &mut extra,
                    );
                }
            }
            // Braced expr arguments.
            for (i, arg) in cmd.argv.iter().enumerate().skip(1) {
                if arg.kind == tcl_lexer::TokenType::Str && expr_idx.contains(&(i - 1)) {
                    collect_expr_reads(
                        &self.source,
                        &self.result,
                        *arg,
                        registry,
                        lexer_config,
                        &mut extra,
                    );
                }
            }
            // Registry `VarRead`-role name words of this command itself
            // (`info exists m`, `array get m`) — a use site of `m` exactly as
            // `$m` is, and one the `$`-token walk above cannot see because
            // there is no `$` to see.
            collect_name_role_reads(&self.source, &self.result, cmd, &mut extra);
            for site in extra {
                self.record_var_read_braced_in_origin(
                    &site.name,
                    site.span,
                    scope_path,
                    site.braced_literal,
                    site.substitution,
                );
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
            .unwrap_or_else(|| crate::naming::qualify("::", cmd_name))
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
    /// it inherits `E`'s own earliest reachable offset
    /// (`proc helper {}`, `proc inner {} { helper }`, `proc outer {} { inner
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
    /// and by [`crate::command_binding::SourceInvocationBinding::selected_slot_presence`] as the "was this call's enclosing
    /// definition itself reached before the deletion" escape hatch.
    ///
    /// # A body edge may raise a callee's offset but never lower a base one
    ///
    /// A call site's *presence* in a body is not proof that the body reaches
    /// it. `proc a {} { if {0} { b } }` never calls `b`, so `a`'s own
    /// earliest offset says nothing about when `b` first runs — yet the
    /// unrestricted fixpoint would hand `b` (and everything `b` calls) `a`'s
    /// early offset, which then reads as "reached before the deletion" and
    /// silently withdraws a correct W123.
    ///
    /// Oracle (tclsh8.6): with `proc b {}
    /// { helper }`, `proc a {} { if {0} { b } }`, `a`, `rename helper {}`,
    /// `b` — the final `b` really does fail with `invalid command name
    /// "helper"`.
    ///
    /// The precise rule would drop edges inside regions a constant-branch
    /// analysis proves unreachable, but those facts do not exist yet here:
    /// SCCP runs later, over the IR this analyser's result feeds, so
    /// consulting it at edge-collection time would be circular. What is
    /// available is the base case itself, so the fixpoint keeps a
    /// **weaker, non-circular** rule:
    ///
    /// > a body edge may only ever *add* a callee offset, never *lower* a
    /// > base top-level one.
    ///
    /// A callee with its own top-level call site has an offset that is
    /// already a fact about real execution; a speculative path through some
    /// body may not undercut it. A callee with no top-level call site keeps
    /// the optimistic answer — the chain is the only evidence there is.
    ///
    /// This is deliberately approximate in the sound direction for the
    /// chained-call shapes above and deliberately optimistic elsewhere. It does not
    /// catch a dead edge to a callee that is never called at top level at
    /// all, and it can raise an offset for a *live* body edge whose callee
    /// also has a later top-level call.
    fn compute_reachable_call_offsets(&self) -> HashMap<String, u32> {
        let mut reachable: HashMap<String, u32> = HashMap::new();
        // Callees with a top-level (non-body) call site. Their offsets are
        // observations of real execution, so body edges may not lower them.
        let mut has_base_offset: HashSet<&str> = HashSet::new();
        // Call-graph edges: `enclosing definition -> callee`, for every
        // invocation whose own site sits inside a body.
        let mut body_edges: Vec<(&str, &str)> = Vec::new();
        for inv in &self.result.command_invocations {
            let Some(qualified) = inv.resolved_qualified_name.as_deref() else {
                continue;
            };
            let call_off = inv.range.start();
            if !self.result.offset_is_inside_any_definition_body(call_off) {
                has_base_offset.insert(qualified);
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
                    Some(_) if has_base_offset.contains(callee) => {}
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

    /// The implicit `namespace path` each recorded call site carries, indexed
    /// alongside `command_invocations`.
    ///
    /// Computed up front, before [`Self::finalise_invocation_resolutions`]
    /// borrows that vector mutably, because the lookup reads the scope tree
    /// living on the same `result`.
    fn implicit_invocation_namespace_paths(&self) -> Vec<&'static [&'static str]> {
        self.result
            .command_invocations
            .iter()
            .map(|inv| {
                implicit_command_namespace_path_at(&self.result.global_scope, inv.range.start())
            })
            .collect()
    }

    /// Preserve actual implementation origins independently of name assistance.
    /// A document-final declaration or lexical deletion gate cannot prove which
    /// implementation a reached call selected after evaluating its arguments.
    pub(super) fn retain_positioned_command_definitions(&mut self) {
        let lexical_advice = self.result.allows_lexical_declaration_advice();
        let completed = self.result.original_command_world.clone();
        for invocation in &mut self.result.command_invocations {
            if invocation.is_mathfunc_call || invocation.callback_arity.is_some() {
                continue;
            }
            let Some(offset) = invocation.lookup.offset(invocation.range) else {
                continue;
            };
            let point = self.head_identities.invocation_at_source("", offset);
            let reference = match invocation.lookup {
                // The ensemble looks up its selected target, after argv.
                crate::signature_scan::types::SignatureCommandLookup::EnsembleDispatch {
                    ..
                } => invocation
                    .resolved_qualified_name
                    .as_deref()
                    .and_then(|target| point.command_reference(target)),
                // A unanimously consumed name is looked up by this consuming
                // command's handler, so the receipt is issued at its site.
                crate::signature_scan::types::SignatureCommandLookup::ConsumedName { .. } => {
                    point.command_reference(&invocation.name)
                }
                crate::signature_scan::types::SignatureCommandLookup::PossibleConsumedName {
                    ..
                } => point
                    .lookup_command_word(&invocation.name)
                    .command_reference(&invocation.name),
                crate::signature_scan::types::SignatureCommandLookup::InvocationHead
                    if point.original_receiver_namespace_commands().is_some() =>
                {
                    completed.as_ref().and_then(|world| {
                        point.original_receiver_command_reference_after_load(world)
                    })
                }
                crate::signature_scan::types::SignatureCommandLookup::InvocationHead => {
                    point.original_evaluated_command_reference().or_else(|| {
                        lexical_advice
                            .then(|| point.evaluated_command_reference())
                            .flatten()
                    })
                }
                crate::signature_scan::types::SignatureCommandLookup::EnsembleTarget { .. }
                | crate::signature_scan::types::SignatureCommandLookup::DeferredReference => None,
            };
            let Some(reference) = reference else {
                invocation.clear_positioned_reference();
                continue;
            };
            invocation.retain_reference(&reference);
        }
        self.retain_ensemble_target_references();
    }

    /// An ensemble target word names the command each dispatch to it looks
    /// up; it shares their receipt only when every such dispatch has one and
    /// all agree.
    fn retain_ensemble_target_references(&mut self) {
        let mut agreed: std::collections::HashMap<
            String,
            Option<crate::command_binding::SourceCommandReference>,
        > = std::collections::HashMap::new();
        for invocation in &self.result.command_invocations {
            if invocation.ensemble_dispatch.is_none() {
                continue;
            }
            let Some(target) = &invocation.resolved_qualified_name else {
                continue;
            };
            let reference = invocation.resolved_command_reference.as_ref();
            agreed
                .entry(target.clone())
                .and_modify(|known| {
                    if !known
                        .as_ref()
                        .zip(reference)
                        .is_some_and(|(known, reference)| known.selects_same_command(reference))
                    {
                        *known = None;
                    }
                })
                .or_insert_with(|| {
                    reference.map(
                        crate::command_binding::SourceCommandReference::without_invocation_site,
                    )
                });
        }
        for invocation in &mut self.result.command_invocations {
            if !matches!(
                invocation.lookup,
                crate::signature_scan::types::SignatureCommandLookup::EnsembleTarget { .. }
            ) {
                continue;
            }
            match invocation
                .resolved_qualified_name
                .as_ref()
                .and_then(|target| agreed.get(target))
            {
                Some(Some(reference)) => invocation.retain_reference(reference),
                _ => invocation.clear_positioned_reference(),
            }
        }
    }

    fn finalise_original_invocation_inputs(&mut self) {
        let config = self.file_lexer_config();
        let rules = self.word_rules();
        let policy = self.declaration_name_policy();
        for invocation in &mut self.result.command_invocations {
            if invocation.matches_by_receipt() {
                if invocation.original_name_input.is_none()
                    && let Some(policy) = policy
                {
                    invocation.original_name_input = ensemble_word_input(
                        &self.head_identities,
                        invocation,
                        config,
                        rules,
                        policy,
                    );
                }
                continue;
            }
            if invocation.is_mathfunc_call || invocation.callback_arity.is_some() {
                continue;
            }
            let Some(offset) = invocation.lookup.offset(invocation.range) else {
                continue;
            };
            let binding = self.head_identities.invocation_at_source("", offset);
            if invocation.original_name_input.is_none() && !invocation.indirect {
                let Some(policy) = policy else {
                    continue;
                };
                invocation.original_name_input = binding
                    .original_source_name_key_at_span(invocation.range, config, rules, policy)
                    .map(crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord);
            }
            if invocation.original_lookup.is_none() {
                let key = invocation.original_name_input.as_ref().and_then(
                    crate::signature_scan::scope::SignatureSourceNameInput::original_word_key,
                );
                // A head is looked up at its own invocation; a unanimously
                // consumed name operand at its consuming command, after argv.
                invocation.original_lookup = match invocation.lookup {
                    crate::signature_scan::types::SignatureCommandLookup::InvocationHead => {
                        key.and_then(|key| binding.original_static_command_lookup(key))
                    }
                    crate::signature_scan::types::SignatureCommandLookup::ConsumedName {
                        ..
                    } => key.and_then(|key| binding.original_consumed_command_lookup(key)),
                    crate::signature_scan::types::SignatureCommandLookup::PossibleConsumedName {
                        ..
                    }
                    | crate::signature_scan::types::SignatureCommandLookup::EnsembleDispatch {
                        ..
                    }
                    | crate::signature_scan::types::SignatureCommandLookup::EnsembleTarget {
                        ..
                    }
                    | crate::signature_scan::types::SignatureCommandLookup::DeferredReference => {
                        None
                    }
                };
            }
        }
    }

    pub(super) fn finalise_invocation_resolutions(&mut self) {
        self.finalise_original_invocation_inputs();
        // Populate the per-dialect builtin-name cache before splitting field
        // borrows below (`builtin_command_names` needs `&mut self`).
        let _ = self.builtin_command_names();
        let Some(builtins) = self.builtin_names.as_ref() else {
            return;
        };
        // `&'static DialectProfile`, not a borrow of `self` — safe to read
        // after the field-splitting `result` borrow below, unlike an `&self`
        // method call (which is why the math-function existence check the
        // mathfunc branch uses is a free function, not `Analyser::profile`
        // plus an instance method).
        let dialect = self.profile;
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
        // `live_for_call`'s body-call escape hatch can consult it, and cached
        // on `self` so the later W123 /
        // const-dispatch / variable-command passes' `Self::fact_live_for_call`
        // calls reuse the same map instead of rebuilding it.
        self.reachable_call_offsets = self.compute_reachable_call_offsets();
        let implicit_paths = self.implicit_invocation_namespace_paths();
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
        for (idx, inv) in result.command_invocations.iter_mut().enumerate() {
            let implicit_path = implicit_paths.get(idx).copied().unwrap_or(&[]);
            // Absolute names are exact — the sole candidate is the name itself.
            // `None` means a background scan that skipped the scope walk; leave
            // its (empty) candidate list untouched.
            if tcl_syntax::naming::is_absolute(&inv.name) {
                inv.resolution_candidates = vec![inv.name.clone()];
                inv.resolved_user_definition =
                    known_ctx.user_definition_known(&inv.name, inv.range.start());
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
                let global = tcl_registry::mathfunc::qualified_name(&inv.name);
                let ns: String = match resolved.strip_suffix(global.as_str()) {
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
                let rel = crate::naming::unroot_rooted_key(&global).unwrap_or(global.as_str());
                let candidates = crate::naming::command_resolution_candidates(&ns, path, rel);
                inv.resolution_candidates.clone_from(&candidates);
                let call_off = inv.range.start();
                let winner = candidates.into_iter().find(|c| {
                    known_ctx.known(c, call_off)
                        || (*c == global
                            && crate::tcl_expr_eval::is_known_mathfunc_in_dialect(
                                &inv.name, dialect,
                            ))
                });
                if let Some(w) = winner {
                    record_known_winner(inv, &resolved, &w, &known_ctx, call_off);
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
            let written = crate::naming::canonical_written_command(&inv.name);
            let suffix = crate::naming::root_unrooted_key(&written);
            let ns: String = match resolved.strip_suffix(suffix.as_str()) {
                Some("") => "::".to_owned(),
                Some(prefix) => prefix.to_owned(),
                // Not the `{ns}::{name}` shape the walk produces.  A folded
                // dynamic head (`${ns}::setdef`) is exactly this case — the
                // written name is not the dispatched one — and the walk
                // already recorded its candidate list from the *folded* name,
                // so settle against that instead of discarding it (without
                // this the local-first guess
                // `::tk::tk::setdef` was never demoted to the global
                // `::tk::setdef` the call really reaches, and find-references
                // from the declaration missed the site).  An unusual spelling
                // with no recorded candidates falls back to the settled
                // name as the sole candidate.
                None => {
                    settle_prebuilt_candidates(inv, resolved, &known_ctx);
                    continue;
                }
            };
            // A frame carrying an *implicit* `namespace path` (a `TclOO`
            // member body's `::oo::Helpers`, registry data — see
            // [`implicit_command_namespace_path_at`]) searches that path
            // between its own current namespace and global.  That current
            // namespace is the object's (`::oo::ObjN`) and has no static
            // spelling, so the walk substitutes global for it — which, left
            // alone, would put the *global* candidate ahead of the path one
            // and invert Tcl's order (tclsh 8.6.16 / 9.0.4: with both
            // `::callback` and `::oo::Helpers::callback` defined, a bare
            // `callback` in a method body reaches the latter).  The
            // unspellable slot is therefore dropped and the implicit path
            // takes its place at the head of the search.
            let declared_path: &[String] = paths.get(&ns).map_or(&[], Vec::as_slice);
            let (search_ns, search_path): (&str, Vec<&str>) = match implicit_path.split_first() {
                Some((first, rest)) => (first, rest.to_vec()),
                None => (
                    ns.as_str(),
                    declared_path.iter().map(String::as_str).collect(),
                ),
            };
            // Record the full ordered candidate list so a cross-document
            // consumer can re-settle this call against a *workspace-wide*
            // existence check — the local-first guess below only sees this
            // file, so a call resolving (via `namespace path`) to a proc
            // defined in another file cannot settle correctly here.
            inv.resolution_candidates =
                crate::naming::command_resolution_candidates(search_ns, &search_path, &inv.name);
            let call_off = inv.range.start();
            let known_here = |c: &str| known_ctx.known(c, call_off);
            if let Some(winner) =
                crate::naming::resolve_command_with(search_ns, &search_path, &inv.name, &known_here)
            {
                record_known_winner(inv, &resolved, &winner, &known_ctx, call_off);
            }
            // No candidate known in this file: keep the local-first guess for
            // cross-file consumers (the reference matchers treat it, and the
            // candidate list above, as candidates rather than ground truth).
        }
        // The constant-`$cmd` dispatch sites are *not* settled here:
        // their value facts come from the compiler's flow-sensitive value
        // model, which needs the CFG/SSA `CompilationUnit` — see
        // `settle_const_dispatches` in the diagnostics phase.  A lexical
        // constant map is last-write-wins across `if`/loop joins and discards
        // the defining literal's writable span, so it cannot answer here.
    }
}

/// Settle an invocation whose candidate list was built at **walk** time
/// rather than by [`Analyser::finalise_invocation_resolutions`] — a folded
/// dynamic head, whose written name is not the dispatched one and so does
/// not carry the `{ns}::{name}` shape the namespace recovery needs.
///
/// The first known candidate wins, exactly as
/// [`crate::naming::resolve_command_with`] would.  With no recorded
/// candidates this keeps the pre-existing behaviour: the settled name is the
/// sole candidate.  Without the settlement the walk's local-first guess
/// `::tk::tk::setdef` is never demoted to the global `::tk::setdef` the call
/// really reaches, so find-references from the declaration misses the site
/// while go-to-definition finds it.
fn settle_prebuilt_candidates<A>(
    inv: &mut crate::signature_scan::types::SignatureCommandInvocation,
    resolved: String,
    known_ctx: &KnownPredicateCtx<'_, A>,
) {
    if inv.resolution_candidates.is_empty() {
        inv.resolution_candidates = vec![resolved];
        return;
    }
    let call_off = inv.range.start();
    let winner = inv
        .resolution_candidates
        .iter()
        .find(|c| known_ctx.known(c, call_off))
        .cloned();
    if let Some(winner) = winner {
        record_known_winner(inv, &resolved, &winner, known_ctx, call_off);
    }
}

#[derive(Debug, Clone)]
pub(super) struct LogicalConstStringProvenance {
    image: tcl_lexer::SourceImage,
    input: super::ResolvedAnalysisInput,
    declaration: Span,
    value: String,
    value_span: Span,
}

#[derive(Debug, Clone)]
pub(super) struct LogicalInterpreterPathProvenance {
    image: tcl_lexer::SourceImage,
    input: super::ResolvedAnalysisInput,
    declaration: Span,
    producer: Span,
    key: String,
}

impl Analyser {
    /// Record a constant string assignment for `var_name` in the
    /// scope at `scope_path`.
    pub fn set_const_string(
        &mut self,
        var_name: &str,
        value: String,
        value_span: Span,
        scope_path: &[usize],
    ) {
        let provenance = self.result.resolved_input.as_ref().and_then(|input| {
            let declaration = scope_at(&self.result.global_scope, scope_path)?
                .variables
                .get(var_name)?;
            let image = self
                .result
                .retained_command_realm()?
                .original_source_image()?
                .clone();
            (self.result.allows_lexical_declaration_advice() && declaration.link_target.is_none())
                .then(|| LogicalConstStringProvenance {
                    image,
                    input: input.clone(),
                    declaration: declaration.definition_span,
                    value: value.clone(),
                    value_span,
                })
        });
        let records = self
            .logical_const_string_provenance
            .entry(scope_path.to_vec())
            .or_default();
        if let Some(provenance) = provenance {
            records.insert(var_name.to_owned(), provenance);
        } else {
            records.remove(var_name);
        }
        self.const_strings
            .entry(scope_path.to_vec())
            .or_default()
            .insert(var_name.to_string(), (value, value_span));
        // Conditional writes cannot establish a value after the branch joins.
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
        if let Some(map) = self.logical_const_string_provenance.get_mut(scope_path) {
            map.remove(var_name);
        }
        if let Some(map) = self.const_strings.get_mut(scope_path) {
            map.remove(var_name);
        }
        if let Some(set) = self.nondominating_consts.get_mut(scope_path) {
            set.remove(var_name);
        }
    }

    /// A dominating value from the exact declaration selected by Logical source advice.
    /// Linked cells and array elements require independent value provenance.
    pub(super) fn lookup_logical_const_string_at(
        &self,
        written: &str,
        span: Span,
    ) -> Option<(&str, Span)> {
        let image = tcl_lexer::SourceImage::document(&self.source);
        let advice = self.result.logical_variable_scope_advice(
            &image,
            self.file_lexer_config(),
            span.start(),
        )?;
        let declaration = advice.variable_definition(written)?;
        if declaration.link_target.is_some() || !declaration.array_indices.is_empty() {
            return None;
        }
        let input = self.result.resolved_input.as_ref()?;
        let mut candidates =
            self.logical_const_string_provenance
                .iter()
                .flat_map(|(path, records)| {
                    records.iter().filter_map(move |(name, provenance)| {
                        let scope = scope_at(&self.result.global_scope, path)?;
                        let retained = scope.variables.get(name)?;
                        if !std::ptr::eq(retained, declaration)
                            || provenance.declaration != declaration.definition_span
                            || &provenance.input != input
                            || !self.result.matches_original_source_image(
                                &provenance.image,
                                self.file_lexer_config(),
                            )
                            || self
                                .nondominating_consts
                                .get(path)
                                .is_some_and(|names| names.contains(name))
                        {
                            return None;
                        }
                        let (value, value_span) = self.const_strings.get(path)?.get(name)?;
                        (value == &provenance.value
                            && *value_span == provenance.value_span
                            && value_span.end() < span.start())
                        .then_some((value.as_str(), *value_span))
                    })
                });
        let value = candidates.next()?;
        candidates.next().is_none().then_some(value)
    }

    /// A scalar substitution retains its complete original word and braced-name form.
    pub(super) fn lookup_logical_const_substitution(&self, token: Token) -> Option<(&str, Span)> {
        if token.kind != TokenType::Var {
            return None;
        }
        let word = self.retained_value_word(token)?;
        let arena = word.executable_parts();
        let [component] = arena.list(arena.root()) else {
            return None;
        };
        let (name, span) = self.logical_variable_value_component(arena, component)?;
        self.lookup_logical_const_string_at(&name, span)
    }

    pub(super) fn retained_value_word(&self, token: Token) -> Option<tcl_lexer::NativeWord> {
        let config = self.file_lexer_config();
        let image = crate::registry_invocation::source_structure::original_query_document_image(
            &self.source,
            &self.result,
            config,
        );
        self.result
            .matches_original_source_image(&image, config)
            .then_some(())?;
        self.result
            .retained_command_realm()?
            .original_written_word_at_span_in_source(&image, token.span, config)
            .map(|(word, _)| word)
    }

    pub(super) fn logical_variable_value_component(
        &self,
        arena: &tcl_lexer::ExecutablePartArena,
        component: &tcl_lexer::SpannedExecutablePart,
    ) -> Option<(String, Span)> {
        if !self.result.allows_lexical_declaration_advice()
            || !self
                .result
                .matches_original_source_image(arena.image(), arena.config())
            || arena.image().bytes() != self.source.as_bytes()
        {
            return None;
        }
        let reference = tcl_lexer::word_parts::scan_var_ref(
            arena.image().bytes(),
            usize::try_from(component.span.start()).ok()?,
            arena.config(),
        )
        .ok()??;
        (u32::try_from(reference.next).ok()? == component.span.end()).then_some(())?;
        let span = reference.source_span(
            arena.image().bytes(),
            usize::try_from(component.span.start()).ok()?,
            0,
        )?;
        let root = crate::signature_scan::variable_name::SignatureSourceVariableRoot::from_original_logical_source_executable(
            arena, arena.image(), arena.config(), span, self.word_rules(), self.declaration_name_policy()?,
        )?;
        if root.is_separate_array_root() {
            return None;
        }
        Some((root.display()?.to_owned(), span))
    }

    pub(super) fn lookup_logical_const_component(
        &self,
        arena: &tcl_lexer::ExecutablePartArena,
        component: &tcl_lexer::SpannedExecutablePart,
    ) -> Option<String> {
        let (name, span) = self.logical_variable_value_component(arena, component)?;
        self.lookup_logical_const_string_at(&name, span)
            .map(|(value, _)| value.to_owned())
    }

    pub(super) fn resolve_logical_value_word(&self, token: Token) -> Option<String> {
        let word = self.retained_value_word(token)?;
        let bytes = tcl_syntax::word_rules::original_word_unicode_value_with_variables(
            &word,
            |arena, component| {
                self.lookup_logical_const_component(arena, component)
                    .map(String::into_bytes)
            },
        )?;
        String::from_utf8(bytes).ok()
    }

    /// The value the interpretation froze for the whole word at `token`, on
    /// a profile whose words it evaluates. A word it never reached, or
    /// reached with two values, has none.
    pub(super) fn frozen_value_word(&self, token: Token) -> Option<String> {
        let config = self.file_lexer_config();
        let image = crate::registry_invocation::source_structure::original_query_document_image(
            &self.source,
            &self.result,
            config,
        );
        self.result
            .matches_original_source_image(&image, config)
            .then_some(())?;
        self.result
            .retained_command_realm()?
            .frozen_written_word_value_at_span_in_source(&image, token.span, config)
    }

    pub(super) fn retain_readonly_dictionary_keys(
        &mut self,
        value: &str,
        value_span: Span,
        write_span: Span,
        receiver_scope: &[usize],
    ) {
        let Ok(elements) = tcl_syntax::list::split_list_in(value, self.file_lexer_config().escapes)
        else {
            return;
        };
        if elements.len() % 2 != 0 {
            return;
        }
        let Some(input) = self.result.resolved_input.clone() else {
            return;
        };
        let Some(image) = self
            .result
            .retained_command_realm()
            .and_then(|realm| realm.original_source_image())
            .cloned()
        else {
            return;
        };
        let config = self.file_lexer_config();
        let original = self
            .result
            .retained_command_realm()
            .and_then(|realm| {
                realm.original_written_word_at_span_in_source(&image, value_span, config)
            })
            .and_then(|(word, _)| {
                let content = word.content_span().ok()?;
                (image.bytes().get(content.as_range()) == Some(value.as_bytes())).then_some(content)
            });
        let original_keys = original_dictionary_key_spans(value, original);
        let declarations = elements
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                let name = pair[0].to_string();
                let declaration = self
                    .result
                    .logical_variable_scope_advice(&image, config, write_span.start())
                    .and_then(|advice| advice.variable_definition(&name))
                    .map(|definition| definition.definition_span);
                (name, declaration)
            })
            .collect::<std::collections::HashMap<_, _>>();
        for declaration in declarations.values().flatten() {
            for (path, records) in &mut self.logical_const_string_provenance {
                let changed = records
                    .iter()
                    .filter(|(_, record)| record.declaration == *declaration)
                    .map(|(name, _)| name.clone())
                    .collect::<Vec<_>>();
                for name in changed {
                    records.remove(&name);
                    if let Some(values) = self.const_strings.get_mut(path) {
                        values.remove(&name);
                    }
                }
            }
        }
        for declaration in declarations.values().flatten() {
            for (path, records) in &mut self.logical_interp_path_provenance {
                let changed = records
                    .iter()
                    .filter(|(_, record)| record.declaration == *declaration)
                    .map(|(name, _)| name.clone())
                    .collect::<Vec<_>>();
                for name in changed {
                    records.remove(&name);
                    if let Some(values) = self.interp_var_bindings.get_mut(path) {
                        values.remove(&name);
                    }
                }
            }
        }
        let Some(scope) = scope_at_mut(&mut self.result.global_scope, receiver_scope) else {
            return;
        };
        for pair in elements.as_chunks::<2>().0 {
            let name = pair[0].to_string();
            scope.readonly_dictionary_keys.insert(
                name.clone(),
                ReadonlyDictionaryKeyVariable {
                    original_key: original_keys.get(&name).copied().flatten(),
                    original_declaration: declarations.get(&name).copied().flatten(),
                    name,
                    image: image.clone(),
                    input: input.clone(),
                    receiver_scope: receiver_scope.to_vec(),
                },
            );
        }
    }

    /// Record `set VAR [interp create ...]`'s resolved interpreter-domain
    /// `key` as `var_name`'s value in the scope at `scope_path` — the
    /// interpreter-value-flow analogue of
    /// [`Self::set_const_string`], scope-chain-aware for the same reason:
    /// two unrelated procs binding the same variable name to different
    /// interpreters must never collide.
    pub fn set_interp_var_binding(&mut self, var_name: &str, key: String, scope_path: &[usize]) {
        self.logical_interp_path_provenance
            .entry(scope_path.to_vec())
            .or_default()
            .remove(var_name);
        self.interp_var_bindings
            .entry(scope_path.to_vec())
            .or_default()
            .insert(var_name.to_string(), key);
    }

    pub(super) fn set_original_logical_interp_var_binding(
        &mut self,
        var_name: &str,
        key: String,
        producer: Token,
        scope_path: &[usize],
    ) {
        self.set_interp_var_binding(var_name, key.clone(), scope_path);
        let provenance = (|| {
            if !self.result.allows_lexical_declaration_advice() || self.conditional_depth != 0 {
                return None;
            }
            let declaration = scope_at(&self.result.global_scope, scope_path)?
                .variables
                .get(var_name)?;
            if declaration.link_target.is_some() || !declaration.array_indices.is_empty() {
                return None;
            }
            self.retained_value_word(producer)?;
            Some(LogicalInterpreterPathProvenance {
                image: self
                    .result
                    .retained_command_realm()?
                    .original_source_image()?
                    .clone(),
                input: self.result.resolved_input.as_ref()?.clone(),
                declaration: declaration.definition_span,
                producer: producer.span,
                key,
            })
        })();
        if let Some(provenance) = provenance {
            self.logical_interp_path_provenance
                .entry(scope_path.to_vec())
                .or_default()
                .insert(var_name.to_owned(), provenance);
        }
    }

    pub(super) fn lookup_logical_interp_path(&self, token: Token) -> Option<String> {
        let word = self.retained_value_word(token)?;
        let arena = word.executable_parts();
        let [component] = arena.list(arena.root()) else {
            return None;
        };
        let (name, span) = self.logical_variable_value_component(arena, component)?;
        let image = tcl_lexer::SourceImage::document(&self.source);
        let advice = self.result.logical_variable_scope_advice(
            &image,
            self.file_lexer_config(),
            span.start(),
        )?;
        let declaration = advice.variable_definition(&name)?;
        if declaration.link_target.is_some() || !declaration.array_indices.is_empty() {
            return None;
        }
        let input = self.result.resolved_input.as_ref()?;
        let mut found = self
            .logical_interp_path_provenance
            .iter()
            .flat_map(|(path, records)| {
                records.iter().filter_map(move |(written, provenance)| {
                    let selected = scope_at(&self.result.global_scope, path)?
                        .variables
                        .get(written)?;
                    if !std::ptr::eq(selected, declaration)
                        || provenance.declaration != declaration.definition_span
                        || &provenance.input != input
                        || provenance.producer.end() >= span.start()
                        || !self.result.matches_original_source_image(
                            &provenance.image,
                            self.file_lexer_config(),
                        )
                        || self.interp_var_bindings.get(path)?.get(written)? != &provenance.key
                    {
                        return None;
                    }
                    Some(provenance.key.as_str())
                })
            });
        let key = found.next()?;
        found.next().is_none().then(|| key.to_owned())
    }

    /// Remove interpreter-binding knowledge for `var_name` (reassigned to
    /// something else — the reassigned value stops resolving as an
    /// interpreter handle, exactly as [`Self::clear_const_string`] does
    /// for a reassigned constant string).
    pub fn clear_interp_var_binding(&mut self, var_name: &str, scope_path: &[usize]) {
        if let Some(records) = self.logical_interp_path_provenance.get_mut(scope_path) {
            records.remove(var_name);
        }
        if let Some(map) = self.interp_var_bindings.get_mut(scope_path) {
            map.remove(var_name);
        }
    }

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

    /// Retained declaration context of the innermost method body. A nested
    /// procedure or lambda owns another frame and resets this readonly context.
    #[must_use]
    pub(super) fn current_method_context(
        &self,
        scope_path: &[usize],
    ) -> Option<&super::types::OriginalLexicalMemberContext> {
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
        self.record_var_read_braced(name, read_span, scope_path, false);
    }

    /// [`Self::record_var_read`] for a read whose delimiters make its content
    /// a **literal** name — a `${…}` reference or a brace-quoted word in a
    /// variable-name role (`[set {$n}]`, `info exists {$n}`).
    ///
    /// Braces suppress every substitution, so the content is the name verbatim:
    /// `${$n}` reads the variable *called* `$n`, which Tcl keeps distinct from
    /// `n` (tclsh 9.0.4 / 8.6.14: `set {$n} v; set ${$n}` → `can't read "v"`,
    /// i.e. it read `$n` and got `v`). Stripping the `$` here would land the
    /// reference on the wrong variable and let Find-References / Rename merge
    /// the two.
    pub fn record_var_read_braced(
        &mut self,
        name: &str,
        read_span: Span,
        scope_path: &[usize],
        braced_literal: bool,
    ) {
        self.record_var_read_braced_in_origin(name, read_span, scope_path, braced_literal, false);
    }

    fn record_var_read_braced_in_origin(
        &mut self,
        name: &str,
        read_span: Span,
        scope_path: &[usize],
        braced_literal: bool,
        substitution: bool,
    ) {
        let base_name = crate::naming::normalise_var_name_braced(name, braced_literal);
        self.record_qualified_var_ref(base_name, read_span, scope_path, substitution, false);
        // An `$arr(idx)` read records the element index on the array var.
        let element = crate::naming::split_array_name_braced(name, braced_literal)
            .1
            .map(ToString::to_string);

        if self.result.allows_lexical_declaration_advice() {
            let image = tcl_lexer::SourceImage::document(&self.source);
            let selected_path = self
                .result
                .logical_variable_scope_advice(&image, self.file_lexer_config(), read_span.start())
                .and_then(|advice| {
                    let selected = advice.variable_declaration_scope(base_name)?;
                    ancestor_paths(scope_path).find(|path| {
                        scope_at(&self.result.global_scope, path)
                            .is_some_and(|scope| std::ptr::eq(scope, selected))
                    })
                });
            if let Some(path) = selected_path
                && let Some(var) = scope_at_mut(&mut self.result.global_scope, &path)
                    .and_then(|scope| scope.variables.get_mut(base_name))
            {
                var.push_reference(read_span);
                if let Some(element) = element {
                    var.array_indices.insert(element);
                }
            }
            return;
        }

        // Local scope first.
        let path = scope_path.to_vec();
        let base_owned = base_name.to_string();
        if let Some(scope) = scope_at_mut(&mut self.result.global_scope, &path)
            && let Some(var) = scope.variables.get_mut(&base_owned)
        {
            var.push_reference(read_span);
            if let Some(e) = element {
                var.array_indices.insert(e);
            }
            return;
        }

        // Cross-rule variables — fall back to global scope.
        if tcl_syntax::naming::is_absolute(&base_owned)
            || tcl_syntax::naming::authored_namespace_hint_tail(&base_owned, "static").is_some()
        {
            if let Some(var) = self.result.global_scope.variables.get_mut(&base_owned) {
                var.push_reference(read_span);
                if let Some(e) = element {
                    var.array_indices.insert(e);
                }
            } else if let Some(captured) = self.capture_global_reads.as_mut() {
                captured.push(super::per_item::CapturedBodyGlobalRead {
                    base_name: base_owned,
                    element,
                    span: read_span,
                });
            }
        } else if !tcl_syntax::naming::is_qualified(base_owned.as_bytes())
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
            var.push_reference(read_span);
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
            .map_or_default(|s| s.variables.keys().cloned().collect())
    }

    /// Record a **namespace-qualified** variable occurrence (see
    /// [`super::types::QualifiedVarRef`]) so the workspace index can build a
    /// cross-document variable reference set.
    ///
    /// `base_name` is the already-normalised name as written (array index
    /// stripped).  Only a name carrying a `::` qualifier is recorded: it names
    /// one cell in one namespace wherever it is written, so a sibling document
    /// can match it without knowing this document's scope chain.  A relative
    /// qualifier (`app::colors::palette`) is rooted against the occurrence's own
    /// command-resolution namespace — the same rule
    /// `tcl_lsp_core::definition::lookup_var_in_scope_chain` applies to the
    /// cursor's side of the same question, so the two can't disagree.
    ///
    /// Recorded whether or not the cell resolves in *this* document: the
    /// cross-file case is precisely the one where it does not.  Repeated calls
    /// for one token (a write bound by both its dedicated handler and the
    /// registry `VarWrite`-role walk) collapse on the trailing duplicate check.
    fn record_qualified_var_ref(
        &mut self,
        base_name: &str,
        span: Span,
        scope_path: &[usize],
        substitution: bool,
        declaration: bool,
    ) {
        use crate::signature_scan::variable_symbol::{
            OriginalVariableSymbolPurpose, OriginalVariableSymbolReceiver,
        };
        let original_name_input = self.original_qualified_variable_input(span, substitution);
        if let Some(crate::signature_scan::scope::SignatureSourceNameInput::OriginalVariableRoot(
            root,
        )) = &original_name_input
            && !self.result.original_variable_roots.contains(root)
        {
            self.result.original_variable_roots.push(root.clone());
        }
        let scope = scope_at(&self.result.global_scope, scope_path);
        let original_namespace = scope.and_then(|scope| scope.naming_scope.clone());
        if !substitution
            && declaration
            && let Some(site) = self
                .result
                .original_variable_symbols
                .iter_mut()
                .find(|site| site.span() == span)
        {
            site.retain_declaration();
        }
        let purpose = if scope.is_some_and(|scope| {
            matches!(
                scope.kind,
                ScopeKind::Global | ScopeKind::Namespace | ScopeKind::Uplevel
            )
        }) {
            OriginalVariableSymbolPurpose::NamespaceStorage
        } else {
            OriginalVariableSymbolPurpose::QualifiedOnly
        };
        if substitution
            && !self
                .result
                .original_variable_symbol_conflicts
                .contains(&span)
            && !self.result.original_variable_alias_sites.contains(&span)
        {
            let site = original_name_input.clone().and_then(|input| {
                self.original_variable_occurrence(
                    span,
                    input,
                    original_namespace.clone(),
                    purpose,
                    OriginalVariableSymbolReceiver::LexicalRoot,
                    declaration,
                )
            });
            if site.is_none() && !self.result.original_variable_name_unknowns.contains(&span) {
                self.result.original_variable_name_unknowns.push(span);
            }
            if let Some(existing) = self
                .result
                .original_variable_symbols
                .iter_mut()
                .find(|existing| existing.span() == span)
            {
                if site
                    .as_ref()
                    .is_none_or(|site| !existing.merge_declaration(site))
                {
                    self.result
                        .original_variable_symbols
                        .retain(|existing| existing.span() != span);
                    self.result.original_variable_symbol_conflicts.push(span);
                }
            } else if let Some(site) = site {
                self.result.original_variable_symbols.push(site);
            }
        }
        if !crate::naming::is_qualified(base_name.as_bytes()) {
            return;
        }
        let qualified = if self.result.allows_lexical_declaration_advice() {
            let image = tcl_lexer::SourceImage::document(&self.source);
            let Some(advice) = self.result.logical_variable_scope_advice(
                &image,
                self.file_lexer_config(),
                span.start(),
            ) else {
                return;
            };
            let Some(qualified) = advice.namespace_variable_coordinate(base_name) else {
                return;
            };
            qualified
        } else {
            let ns = self.command_resolution_namespace(scope_path);
            crate::naming::qualify(&ns, base_name)
        };
        self.retain_qualified_variable_reference(
            qualified,
            span,
            original_name_input,
            original_namespace,
        );
    }

    fn retain_qualified_variable_reference(
        &mut self,
        qualified: String,
        span: Span,
        original_name_input: Option<crate::signature_scan::scope::SignatureSourceNameInput>,
        original_namespace: Option<crate::signature_scan::scope::SignatureNamespaceScope>,
    ) {
        if let Some(last) = self
            .result
            .qualified_var_refs
            .last_mut()
            .filter(|last| last.span == span && last.qualified_name == qualified)
        {
            if last.original_name_input != original_name_input {
                last.original_name_input = None;
            }
            if last.original_namespace != original_namespace {
                last.original_namespace = None;
            }
            return;
        }
        self.result
            .qualified_var_refs
            .push(super::types::QualifiedVarRef {
                qualified_name: qualified,
                span,
                original_name_input,
                original_namespace,
            });
    }

    pub(super) fn record_original_variable_write_advice(
        &mut self,
        span: Span,
        scope_path: &[usize],
        form: tcl_registry::resolved_invocation::VariableReceiverOperandForm,
        conditional_metadata: Option<
            &crate::registry_invocation::OriginalConditionalRegistryMetadata,
        >,
    ) {
        let image = tcl_lexer::SourceImage::document(&self.source);
        let config = self.file_lexer_config();
        if let Some(original) = self
            .result
            .original_vendor_source_name_in_source(&image, config, span)
        {
            let Some(metadata) = self.original_vendor_variable_metadata(original) else {
                return;
            };
            let home =
                self.result
                    .original_vendor_variable_body_in_source(&image, config, span.start());
            let source_root = home.is_none()
                && self.body_depth == 0
                && scope_at(&self.result.global_scope, scope_path)
                    .is_some_and(|scope| scope.kind == ScopeKind::Global);
            let Some(advice) =
                crate::signature_scan::vendor_variable::VendorSourceVariableAdvice::written(
                    original,
                    span,
                    form,
                    home,
                    source_root,
                    metadata,
                )
            else {
                return;
            };
            if !self
                .result
                .original_vendor_variable_advice
                .contains(&advice)
            {
                self.result.original_vendor_variable_advice.push(advice);
            }
            return;
        }
        let Some(input) = self.original_qualified_variable_input(span, false) else {
            return;
        };
        let frame = self
            .head_identities
            .source_bindings_ref()
            .original_variable_frame_at_span(span, self.file_lexer_config())
            .or_else(|| {
                self.head_identities
                    .source_bindings_ref()
                    .original_variable_frame_at_offset(span.start(), self.file_lexer_config())
            });
        let namespace = scope_at(&self.result.global_scope, scope_path)
            .and_then(|scope| scope.naming_scope.clone());
        let context = self.analysis_context();
        let advice = crate::signature_scan::variable_symbol::OriginalVariableWriteAdvice::new(
            span,
            input,
            form,
            frame,
            namespace,
            conditional_metadata.cloned(),
            &context,
        );
        let same_card =
            |existing: &crate::signature_scan::variable_symbol::OriginalVariableWriteAdvice| {
                // Cheapest test first: most cards are at another span.
                existing.span() == advice.span()
                    && existing.receiver_form() == advice.receiver_form()
                    && existing.matches_context(&context)
                    && existing.original_name_input() == advice.original_name_input()
                    && existing.original_frame() == advice.original_frame()
                    && existing.original_namespace() == advice.original_namespace()
            };
        if let Some(existing) = self
            .result
            .original_variable_write_advice
            .iter_mut()
            .find(|existing| same_card(existing))
        {
            if existing.conditional_metadata().is_none() && advice.conditional_metadata().is_some()
            {
                *existing = advice;
            } else if advice.conditional_metadata().is_some() && existing != &advice {
                self.result.original_variable_write_advice.push(advice);
            }
        } else {
            self.result.original_variable_write_advice.push(advice);
        }
    }

    pub(super) fn record_original_vendor_variable_body(
        &mut self,
        declaration_span: Span,
        body_span: Span,
        parameters_span: Option<Span>,
        kind: crate::signature_scan::vendor_variable::VendorSourceVariableBodyKind,
    ) {
        let image = tcl_lexer::SourceImage::document(&self.source);
        let config = self.file_lexer_config();
        let Some(declaration) =
            self.result
                .original_vendor_source_name_in_source(&image, config, declaration_span)
        else {
            return;
        };
        let Some(body) = self
            .result
            .original_vendor_source_name_in_source(&image, config, body_span)
        else {
            return;
        };
        let Some(metadata) = self.original_vendor_variable_metadata(body) else {
            return;
        };
        let parameters = parameters_span.and_then(|span| {
            self.result
                .original_vendor_source_name_in_source(&image, config, span)
        });
        let Some(receipt) = crate::signature_scan::vendor_variable::VendorSourceVariableBody::new(
            declaration,
            body,
            parameters,
            kind,
            metadata,
        ) else {
            return;
        };
        if !self
            .result
            .original_vendor_variable_bodies
            .contains(&receipt)
        {
            for advice in receipt.formal_advice() {
                if !self
                    .result
                    .original_vendor_variable_advice
                    .contains(&advice)
                {
                    self.result.original_vendor_variable_advice.push(advice);
                }
            }
            self.result.original_vendor_variable_bodies.push(receipt);
        }
    }

    pub(super) fn original_vendor_variable_metadata(
        &self,
        original: &crate::signature_scan::vendor_name::VendorSourceNameOccurrence,
    ) -> Option<crate::registry_invocation::VendorRegistryInvocationShape> {
        let head = crate::signature_scan::vendor_name::VendorSourceNameInput::from_original_word(
            original.original_words().first()?,
            original.name_input().policy(),
        );
        crate::registry_invocation::vendor_registry_invocation_shape(
            &self.analysis_context(),
            &head,
            original.original_words(),
        )
    }

    pub(super) fn record_original_variable_receiver(
        &mut self,
        span: Span,
        scope_path: &[usize],
        form: tcl_registry::resolved_invocation::VariableReceiverOperandForm,
        declaration: bool,
    ) {
        use crate::signature_scan::variable_symbol::{
            OriginalVariableSymbolPurpose, OriginalVariableSymbolReceiver,
        };
        if self.result.original_variable_alias_sites.contains(&span)
            || self
                .result
                .original_variable_symbol_conflicts
                .contains(&span)
        {
            return;
        }
        let Some(scope) = scope_at(&self.result.global_scope, scope_path) else {
            return;
        };
        let purpose = if matches!(
            scope.kind,
            ScopeKind::Global | ScopeKind::Namespace | ScopeKind::Uplevel
        ) {
            OriginalVariableSymbolPurpose::NamespaceStorage
        } else {
            OriginalVariableSymbolPurpose::QualifiedOnly
        };
        let namespace = scope.naming_scope.clone();
        let site = self
            .original_qualified_variable_input(span, false)
            .and_then(|input| {
                let namespace = namespace.or_else(|| {
                    let protocol = input.policy().recipe();
                    let tcl_syntax::naming::NativeVariableInputForm::Combined(bytes) =
                        form.input_form(protocol, input.bytes())?
                    else {
                        return None;
                    };
                    let selected = protocol.combined_variable_input(bytes);
                    (selected.root().qualification()
                        == tcl_syntax::naming::NativeNameQualification::Absolute)
                        .then(|| self.original_variable_root_namespace())
                        .flatten()
                });
                self.original_variable_occurrence(
                    span,
                    input,
                    namespace,
                    purpose,
                    OriginalVariableSymbolReceiver::Operand(form),
                    declaration,
                )
            });
        if site.is_none() && !self.result.original_variable_name_unknowns.contains(&span) {
            self.result.original_variable_name_unknowns.push(span);
        }
        if let Some(existing) = self
            .result
            .original_variable_symbols
            .iter_mut()
            .find(|existing| existing.span() == span)
        {
            if site
                .as_ref()
                .is_none_or(|site| !existing.merge_declaration(site))
            {
                self.result
                    .original_variable_symbols
                    .retain(|site| site.span() != span);
                self.result.original_variable_symbol_conflicts.push(span);
            }
        } else if let Some(site) = site {
            self.result.original_variable_symbols.push(site);
        }
    }

    pub(super) fn record_original_formal_variables(&mut self) {
        let Some(policy) = self.declaration_name_policy() else {
            return;
        };
        let input = self.resolved_analysis_input();
        let occurrences = if input.has_logical_source_name_context() {
            self.head_identities
                .original_logical_variable_formal_occurrences(&input)
        } else {
            self.head_identities
                .source_bindings_ref()
                .original_variable_formal_occurrences(self.file_lexer_config())
        };
        for occurrence in occurrences {
            if occurrence.symbol().policy() != policy {
                continue;
            }
            if let Some(existing) = self
                .result
                .original_variable_symbols
                .iter_mut()
                .find(|existing| existing.span() == occurrence.span())
            {
                if !existing.merge_declaration(&occurrence) {
                    self.result
                        .original_variable_symbol_conflicts
                        .push(occurrence.span());
                }
            } else {
                self.result.original_variable_symbols.push(occurrence);
            }
        }
    }

    fn original_variable_occurrence(
        &self,
        span: Span,
        input: crate::signature_scan::scope::SignatureSourceNameInput,
        namespace: Option<crate::signature_scan::scope::SignatureNamespaceScope>,
        purpose: crate::signature_scan::variable_symbol::OriginalVariableSymbolPurpose,
        receiver: crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver,
        declaration: bool,
    ) -> Option<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence> {
        use crate::signature_scan::variable_symbol::{
            OriginalVariableSymbolPurpose, SignatureSourceVariableOccurrence,
        };
        let bindings = self.head_identities.source_bindings_ref();
        let mut aliases = self
            .result
            .original_variable_alias_receipts
            .iter()
            .filter(|alias| {
                self.registry.as_deref().is_some_and(|registry| {
                    bindings.original_variable_alias_matches_at_span(
                        span,
                        &input,
                        receiver,
                        alias,
                        self.file_lexer_config(),
                        registry,
                    )
                })
            });
        if let Some(alias) = aliases.next() {
            if aliases.any(|other| other != alias) {
                return None;
            }
            return Some(
                SignatureSourceVariableOccurrence::from_original_alias_input(
                    span,
                    input,
                    namespace,
                    receiver,
                    alias.clone(),
                ),
            );
        }
        if let Some(source_input) = self.result.resolved_input.as_ref()
            && source_input.has_logical_source_name_context()
        {
            return self.original_logical_variable_occurrence(
                source_input,
                span,
                input,
                receiver,
                declaration,
            );
        }

        if let Some(namespace) = &namespace
            && let Some(site) = SignatureSourceVariableOccurrence::from_original_input(
                span,
                input.clone(),
                namespace.clone(),
                purpose,
                receiver,
                declaration,
            )
        {
            if purpose == OriginalVariableSymbolPurpose::NamespaceStorage {
                return self.original_namespace_storage_occurrence(
                    span,
                    input,
                    namespace,
                    receiver,
                    declaration,
                    site,
                );
            }
            return Some(site);
        }
        (purpose == OriginalVariableSymbolPurpose::QualifiedOnly).then_some(())?;
        let (frame, simple) = self
            .head_identities
            .source_bindings_ref()
            .original_local_variable_symbol_at_span(
                span,
                &input,
                receiver,
                self.file_lexer_config(),
                self.registry.as_deref()?,
            )?;
        Some(
            SignatureSourceVariableOccurrence::from_original_local_input(
                span,
                input,
                namespace,
                frame,
                simple,
                receiver,
                declaration,
            ),
        )
    }

    fn original_logical_variable_occurrence(
        &self,
        source_input: &super::ResolvedAnalysisInput,
        span: Span,
        input: crate::signature_scan::scope::SignatureSourceNameInput,
        receiver: crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver,
        declaration: bool,
    ) -> Option<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence> {
        use crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence;
        let image = tcl_lexer::SourceImage::document(&self.source);
        let scope = self.head_identities.original_logical_variable_scope_at(
            source_input,
            &image,
            span,
            &input,
        )?;
        let mut aliases = self
            .result
            .original_variable_alias_receipts
            .iter()
            .filter(|alias| scope.matches_alias_template(span, &input, receiver, alias));
        if let Some(alias) = aliases.next() {
            if aliases.any(|other| other != alias) {
                return None;
            }
            return SignatureSourceVariableOccurrence::from_logical_alias_input(
                span,
                input,
                receiver,
                alias.clone(),
                scope,
            );
        }
        SignatureSourceVariableOccurrence::from_logical_source(
            span,
            input,
            receiver,
            declaration,
            scope,
            &self.result.original_variable_alias_receipts,
        )
    }

    fn original_namespace_storage_occurrence(
        &self,
        span: Span,
        input: crate::signature_scan::scope::SignatureSourceNameInput,
        namespace: &crate::signature_scan::scope::SignatureNamespaceScope,
        receiver: crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver,
        declaration: bool,
        site: crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence,
    ) -> Option<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence> {
        use crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence;
        let bindings = self.head_identities.source_bindings_ref();
        // Bare names in a namespace frame can select a global table
        // on C8. Only the independently retained point owns that hop.
        let protocol = input.policy().recipe();
        let selected = match receiver.input_form(&input)? {
            tcl_syntax::naming::NativeVariableInputForm::Combined(bytes) => {
                protocol.combined_variable_input(bytes)
            }
            tcl_syntax::naming::NativeVariableInputForm::Separate { root, element } => {
                protocol.separate_variable_input(root, element)
            }
        };
        if matches!(
            protocol.variable_root_geometry(namespace.context()?, selected.root().selected()),
            tcl_syntax::naming::NativeVariableRootGeometry::Local(_)
        ) {
            if let Some((selected_namespace, simple)) = bindings
                .original_namespace_variable_symbol_at_span(
                    span,
                    &input,
                    receiver,
                    self.file_lexer_config(),
                    self.registry.as_deref()?,
                )
            {
                return SignatureSourceVariableOccurrence::from_original_namespace_cell(
                    span,
                    input,
                    namespace.clone(),
                    &selected_namespace,
                    simple,
                    receiver,
                    declaration,
                );
            }
            let (frame, simple) = bindings.original_local_variable_symbol_at_span(
                span,
                &input,
                receiver,
                self.file_lexer_config(),
                self.registry.as_deref()?,
            )?;
            return Some(
                SignatureSourceVariableOccurrence::from_original_local_input(
                    span,
                    input,
                    Some(namespace.clone()),
                    frame,
                    simple,
                    receiver,
                    declaration,
                ),
            );
        }
        Some(site)
    }

    pub(super) fn original_qualified_variable_input(
        &self,
        span: Span,
        substitution: bool,
    ) -> Option<crate::signature_scan::scope::SignatureSourceNameInput> {
        use crate::signature_scan::scope::SignatureSourceNameInput;
        let policy = self.declaration_name_policy()?;
        let config = self.file_lexer_config();
        if !substitution && self.selected_logical_declaration_advice() {
            let image = tcl_lexer::SourceImage::document(&self.source);
            return self
                .head_identities
                .original_written_word_at_span_in_source(&image, span, config)
                .and_then(|(word, _)| {
                    crate::signature_scan::scope::SignatureSourceNameKey::from_original_logical_source_word(
                        &word, self.word_rules(), policy,
                    )
                })
                .map(SignatureSourceNameInput::OriginalWord)
                .or_else(|| {
                    let (word, _) = self.head_identities.original_written_word_at_span_in_source(&image, span, config)?;
                    crate::signature_scan::scope::SignatureSourceNameValue::from_original_variable_operand_root(
                        &word, self.word_rules(), policy,
                    ).map(SignatureSourceNameInput::OriginalValue)
                })
                .or_else(|| self.original_static_variable_operand_input(span))
                .filter(|input| input.policy() == policy);
        }
        if !substitution {
            return self
                .head_identities
                .original_written_name_input_at_span(span, config)
                .or_else(|| self.original_static_variable_operand_input(span))
                .or_else(|| {
                    self.head_identities
                        .original_c_array_operand_root_at_span_in_source(
                            &tcl_lexer::SourceImage::document(&self.source),
                            span,
                            config,
                            policy,
                        )
                })
                .filter(|input| input.policy() == policy);
        }
        let image = tcl_lexer::SourceImage::document(&self.source);
        let start = usize::try_from(span.start()).ok()?;
        let reference =
            tcl_lexer::word_parts::scan_var_ref(image.bytes(), start, config).ok()??;
        (reference.source_span(image.bytes(), start, 0)? == span).then_some(())?;
        let raw_span = Span::new(span.start(), u32::try_from(reference.next).ok()?);
        let arena = tcl_lexer::ExecutablePartArena::decompose(
            image.clone(),
            raw_span,
            tcl_lexer::word_parts::SubstFlags::default(),
            config,
        )
        .ok()?;
        let root = if self.selected_logical_declaration_advice() {
            crate::signature_scan::variable_name::SignatureSourceVariableRoot::from_original_logical_source_executable(
                &arena, &image, config, span, self.word_rules(), policy,
            )
        } else {
            crate::signature_scan::variable_name::SignatureSourceVariableRoot::from_original_executable(
                &arena, &image, config, span, self.word_rules(), policy,
            )
        };
        root.map(SignatureSourceNameInput::OriginalVariableRoot)
    }

    /// Post-walk pass: attach each recorded [`super::types::QualifiedVarRef`]
    /// to the [`VarDef`] of the cell it names, when this document declares
    /// that cell.
    ///
    /// The walk itself cannot do this.  `record_var_read`'s own lookup is
    /// scope-table-local, so a *relative*-qualified occurrence
    /// (`$app::colors::palette` written at global scope, naming
    /// `::app::colors::palette`) finds nothing and is dropped — even when the
    /// declaring `namespace eval` sits in the same file.  Resolving at record
    /// time cannot work either: a qualified read may precede
    /// its declaring `namespace eval` textually and still resolve at run time
    /// (tclsh 9.0.4 / 8.6.16: `proc p {} { return $::n::v }; namespace eval n
    /// { variable v 1 }; puts [p]` prints `1`), so the namespace tables are
    /// only complete once the walk is.
    ///
    /// Cost is one pass over this document's namespace-variable set plus one
    /// over its qualified occurrences — both already collected, neither
    /// per-request.
    pub(super) fn attach_qualified_var_references(&mut self) {
        if self.result.qualified_var_refs.is_empty() {
            return;
        }
        let mut by_cell: HashMap<&str, Vec<Span>> = HashMap::new();
        for vref in &self.result.qualified_var_refs {
            by_cell
                .entry(
                    tcl_syntax::naming::unroot_rooted_key(&vref.qualified_name)
                        .unwrap_or(&vref.qualified_name),
                )
                .or_default()
                .push(vref.span);
        }
        let mut root = std::mem::take(&mut self.result.global_scope);
        attach_qualified_var_refs_to_scope("::", &mut root, &by_cell);
        self.result.global_scope = root;
    }

    /// Record a variable definition in the scope at `scope_path`.
    ///
    /// New definitions are inserted into the scope's `variables` map
    /// and registered into ``result.all_variables`` keyed by
    /// ``"<scope_name>::<base_name>"``.  Re-defining an existing
    /// variable does not overwrite — it only escalates the
    /// `warn_if_unused` flag.
    ///
    /// `tok` is the token of the word that **spells the name**, and its kind
    /// decides how that spelling reads: a brace-quoted word (`TokenType::Str`
    /// — `set {$n} 1`, `foreach {$a} …`) substitutes nothing, so its content
    /// is the literal name, `$` and all.  Tcl keeps `{$n}` and `n` apart
    /// (tclsh 9.0.4 / 8.6.14: `set {$n} v; info exists {$n}` → 1 while
    /// `info exists n` → 0); normalising the `$` away collapsed them into one
    /// `VarDef`, so Find-References answered for both and a rename of `n`
    /// rewrote the `{$n}` word. The array-element suffix still
    /// comes off — `{arr($i)}` is element `$i` of the array `arr`.
    pub fn define_var(
        &mut self,
        name: &str,
        tok: Token,
        scope_path: &[usize],
        warn_if_unused: bool,
        definition_span: Option<Span>,
    ) {
        let braced_literal = tok.kind == TokenType::Str;
        let base_name = crate::naming::normalise_var_name_braced(name, braced_literal);
        if base_name.is_empty() {
            return;
        }
        let base_owned = base_name.to_string();
        let span = definition_span.unwrap_or(tok.span);
        // A structural rebind (the per-item isolated pass re-binding params /
        // instance vars) records no occurrence: the shell walk already did,
        // and the rebind's placeholder token would add a phantom zero-width
        // ref for a `::`-qualified parameter name.
        if !self.structural_rebind {
            self.record_qualified_var_ref(base_name, span, scope_path, false, true);
            // Isolated per-item body: a definition landing in the body's own
            // (throwaway) root scope is a fixed global cell the whole-file
            // walk would have written into the *document's* global scope, so
            // capture it for the graft to replay there — the write-side twin
            // of `capture_global_reads`.
            if scope_path.is_empty()
                && let Some(captured) = self.capture_global_defs.as_mut()
            {
                captured.push((name.to_string(), tok, span));
            }
        }
        // A `set arr(idx) …` definition records the element index on the array.
        let element = crate::naming::split_array_name_braced(name, braced_literal)
            .1
            .filter(|e| !e.is_empty())
            .map(ToString::to_string);

        let path = scope_path.to_vec();
        // First definition of this variable → the W215 reachability check
        // fires here.
        // Done with a short immutable borrow before the mutable scope handle.
        let is_first_def = scope_at(&self.result.global_scope, &path)
            .is_some_and(|s| !s.variables.contains_key(base_name));
        if is_first_def && !self.structural_rebind {
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
            if let Some(global) =
                self.result
                    .all_variables
                    .get_mut(&tcl_syntax::naming::scope_variable_key(
                        &scope.name,
                        &base_owned,
                    ))
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
                link_target_span: None,
            };
            scope
                .original_variable_declaration_spans
                .insert(base_owned.clone(), span);
            scope.variables.insert(base_owned.clone(), var.clone());
            let key = tcl_syntax::naming::scope_variable_key(&scope.name, &base_owned);
            self.result.all_variables.insert(key, var);
        }
    }

    /// Record that the local `name` in the scope at `scope_path` aliases the
    /// namespace/global cell `target` (a `global` / `variable` / `namespace
    /// upvar` binding), so Find-References / Rename can unify every alias of the
    /// same cell.  A no-op if the variable wasn't defined (call after
    /// [`Self::define_var`]).
    ///
    /// `target_span` is the span of the **word that names the cell**, which is
    /// not always the declaration word: `variable v` / `global ::ns::v` name
    /// the cell with the very word that introduces the local alias, but
    /// `namespace upvar ::ns v local` / `upvar #0 ::ns::v local` name it one
    /// word earlier.  Recording it is what lets a rename of the cell rewrite
    /// the right word — see [`VarDef::link_target_span`].
    pub(super) fn set_var_link_target(
        &mut self,
        name: &str,
        scope_path: &[usize],
        target: VariableLinkTarget,
    ) {
        let VariableLinkTarget {
            target,
            target_span,
            local_span,
            original_target_namespace,
            original_target_receiver,
        } = target;
        let _ = self.record_original_variable_alias_target(
            local_span,
            target_span,
            original_target_namespace,
            original_target_receiver,
        );
        self.set_var_link_report(name, scope_path, target, target_span);
    }

    pub(super) fn set_var_link_report(
        &mut self,
        name: &str,
        scope_path: &[usize],
        target: String,
        target_span: Span,
    ) {
        let base = crate::naming::normalise_var_name(name).to_string();
        if let Some(scope) = scope_at_mut(&mut self.result.global_scope, scope_path)
            && let Some(v) = scope.variables.get_mut(&base)
        {
            v.link_target = Some(target);
            v.link_target_span = Some(target_span);
        }
    }

    pub(super) fn record_original_variable_alias_target(
        &mut self,
        local_span: Span,
        target_span: Span,
        original_target_namespace: Option<crate::signature_scan::scope::SignatureNamespaceScope>,
        original_target_receiver: crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver,
    ) -> Option<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence> {
        self.result
            .original_variable_symbols
            .retain(|site| site.span() != local_span && site.span() != target_span);
        self.result.original_variable_alias_sites.push(local_span);
        self.result.original_variable_alias_sites.push(target_span);
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ALIAS_TEMPLATE_SOURCE").is_some() {
            eprintln!(
                "ALIAS_SOURCE_LINK local={} remote={} namespace={} input={}",
                local_span.start(),
                target_span.start(),
                original_target_namespace.is_some(),
                self.original_qualified_variable_input(target_span, false)
                    .is_some()
            );
        }
        let original = original_target_namespace.and_then(|namespace| {
            let name = self.original_qualified_variable_input(target_span, false)?;
            let input = self.resolved_analysis_input();
            if input.has_logical_source_name_context() {
                let scope = self.head_identities.original_logical_variable_scope_at(
                    &input, &tcl_lexer::SourceImage::document(&self.source), target_span, &name,
                )?;
                return crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence::from_logical_alias_target(
                    target_span, name, namespace, original_target_receiver, scope,
                );
            }
            crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence::from_original_input(
                target_span, name, namespace,
                crate::signature_scan::variable_symbol::OriginalVariableSymbolPurpose::NamespaceStorage, original_target_receiver, true,
            )
        });
        if let Some(site) = original.as_ref() {
            if let Some(receipt) = self.original_variable_alias_receipt(local_span, site) {
                self.result
                    .original_variable_name_unknowns
                    .retain(|&span| span != local_span && span != target_span);
                if !self
                    .result
                    .original_variable_alias_receipts
                    .contains(&receipt)
                {
                    self.result.original_variable_alias_receipts.push(receipt);
                }
            }
            self.result.original_variable_symbols.push(site.clone());
        }

        original
    }

    pub(super) fn define_original_namespace_alias_target(
        &mut self,
        occurrence: &crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence,
    ) {
        let Some(key) = occurrence.symbol().checked_c_namespace_key() else {
            return;
        };
        let span = occurrence.span();
        if !occurrence.is_declaration()
            || span.is_empty()
            || !self.result.original_variable_symbols.contains(occurrence)
        {
            return;
        }
        let input = occurrence.original_name_input();
        let selected = input
            .policy()
            .recipe()
            .combined_variable_input(input.bytes());
        let element = selected
            .element()
            .and_then(|element| std::str::from_utf8(element.selected()).ok())
            .filter(|element| !element.is_empty())
            .map(str::to_owned);
        let scope = &mut self.result.global_scope;
        if let Some(variable) = scope.variables.get_mut(&key) {
            if variable.definition_span != span && variable.references.last() != Some(&span) {
                variable.references.push(span);
            }
            if let Some(element) = element {
                variable.array_indices.insert(element);
            }
        } else {
            let variable = VarDef {
                name: key.clone(),
                definition_span: span,
                references: Vec::new(),
                warn_if_unused: false,
                array_indices: element.into_iter().collect(),
                link_target: None,
                link_target_span: None,
            };
            scope
                .original_variable_declaration_spans
                .insert(key.clone(), span);
            scope.variables.insert(key.clone(), variable);
        }
        self.result.all_variables.insert(
            tcl_syntax::naming::scope_variable_key(&scope.name, &key),
            scope.variables[&key].clone(),
        );
    }

    fn original_variable_alias_receipt(
        &self,
        local_span: Span,
        target: &crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence,
    ) -> Option<crate::signature_scan::variable_symbol::OriginalVariableAliasReceipt> {
        use crate::registry_invocation::DeclarationVariableAliasPurpose as Purpose;
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ALIAS_TEMPLATE_SOURCE").is_some() {
            eprintln!(
                "ALIAS_SOURCE_RECEIPT local={} remote={} frame={} input={} operands={:?}",
                local_span.start(),
                target.span().start(),
                self.head_identities
                    .source_bindings_ref()
                    .original_variable_alias_frame_at_span(local_span, self.file_lexer_config())
                    .is_some(),
                self.original_variable_operand_input(local_span).is_some(),
                self.result.original_variable_alias_operands
            );
        }

        let input = self.resolved_analysis_input();
        let image = tcl_lexer::SourceImage::document(&self.source);
        let local_input = self.original_variable_operand_input(local_span)?;
        let (frame, invocation_offset) = if input.has_logical_source_name_context() {
            let mut sources =
                self.result
                    .original_variable_alias_source_operands
                    .iter()
                    .filter(|source| {
                        source.operands().iter().any(|&(local, remote, _)| {
                            local == local_span && remote == target.span()
                        })
                    });
            let source = sources.next()?;
            let offset = source.invocation_offset()?;
            if !source.matches_source_context(
                &image,
                input.lexer_config(),
                input.borrowed_context_registry(),
            ) || sources.any(|other| {
                other.invocation_offset() != Some(offset)
                    || !other.matches_source_context(
                        &image,
                        input.lexer_config(),
                        input.borrowed_context_registry(),
                    )
            }) {
                return None;
            }
            let scope = self.head_identities.original_logical_variable_scope_at(
                &input,
                &image,
                local_span,
                &local_input,
            )?;
            (scope.alias_frame()?, offset)
        } else {
            self.head_identities
                .source_bindings_ref()
                .original_variable_alias_frame_at_span(local_span, self.file_lexer_config())?
        };
        let mut operands = self.result.original_variable_alias_operands.iter().filter(
            |&&(site, local, remote, _)| {
                site == invocation_offset && local == local_span && remote == target.span()
            },
        );
        let &(_, _, _, purpose) = operands.next()?;
        if operands.any(|operand| operand.3 != purpose) {
            return None;
        }
        let context = input.context_registry();
        if self
            .result
            .original_variable_alias_source_operands
            .iter()
            .any(|source| {
                source.invocation_offset() == Some(invocation_offset)
                    && source
                        .operands()
                        .iter()
                        .any(|&(local, remote, _)| local == local_span && remote == target.span())
                    && !source.matches_source_context(&image, self.file_lexer_config(), &context)
            })
        {
            return None;
        }
        if local_input.policy() != target.symbol().policy() {
            return None;
        }
        let local = crate::signature_scan::variable_symbol::original_variable_alias_local_name(
            &local_input,
            purpose,
        )?;
        Some(
            crate::signature_scan::variable_symbol::OriginalVariableAliasReceipt::new(
                frame,
                local,
                target.symbol().clone(),
                target.original_name_input().clone(),
                local_input,
                matches!(purpose, Purpose::Global | Purpose::NamespaceVariable),
                (invocation_offset, local_span),
            ),
        )
    }

    pub(super) fn original_variable_namespace_at(
        &self,
        scope_path: &[usize],
    ) -> Option<crate::signature_scan::scope::SignatureNamespaceScope> {
        scope_at(&self.result.global_scope, scope_path)?
            .naming_scope
            .clone()
    }

    pub(super) fn original_variable_root_namespace(
        &self,
    ) -> Option<crate::signature_scan::scope::SignatureNamespaceScope> {
        Some(crate::signature_scan::scope::SignatureNamespaceScope::root(
            Some(self.declaration_name_policy()?),
        ))
    }

    fn original_static_variable_operand_input(
        &self,
        span: Span,
    ) -> Option<crate::signature_scan::scope::SignatureSourceNameInput> {
        use crate::signature_scan::scope::SignatureSourceNameInput;
        self.head_identities
            .original_source_name_at_span(
                span,
                self.file_lexer_config(),
                self.word_rules(),
                self.declaration_name_policy()?,
            )
            .or_else(|| self.original_static_source_name_at_span(span).cloned())
            .map(|original| SignatureSourceNameInput::OriginalWord(original.name_input().clone()))
    }

    /// Readonly name units from the actual invocation before constructing
    /// source namespace geometry. Static declaration fallbacks keep their
    /// existing independent source/policy owner; no variable receiver or
    /// runtime frame is created by this projection.
    pub(super) fn original_variable_operand_input_at_invocation(
        &self,
        invocation_offset: u32,
        span: Span,
    ) -> Option<crate::signature_scan::scope::SignatureSourceNameInput> {
        // Implementation contract: naming.source.original-point-operand-projection
        // docs/design/analysis/name-resolution-proofs/original-point-operand-projection.md
        let policy = self.declaration_name_policy()?;
        self.head_identities
            .original_written_name_input_at_invocation_span(
                &tcl_lexer::SourceImage::document(&self.source),
                self.file_lexer_config(),
                invocation_offset,
                span,
            )
            .or_else(|| self.original_static_variable_operand_input(span))
            .filter(|input| input.policy() == policy)
    }

    pub(super) fn original_variable_operand_input(
        &self,
        span: Span,
    ) -> Option<crate::signature_scan::scope::SignatureSourceNameInput> {
        self.original_qualified_variable_input(span, false)
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
        use super::types::Severity;
        use crate::naming::is_brace_substitutable;
        use std::borrow::Cow;

        let braced = tok.kind == TokenType::Str;

        // An unbraced word carrying a live `$var` / `[cmd]` substitution
        // (e.g. `set ns::$k 1`) is not a single literal token: `word_piece`
        // re-braces the substitution piece when reconstructing the word's
        // display text (`ns::$k` -> `"ns::${k}"`, so an adjacent literal
        // suffix can't be misread as part of the variable name), and that
        // reconstruction is what `base_name` is built from. Its `{`/`}`
        // characters are reconstruction artefacts, not text the analyser can
        // attribute to the *runtime* name — the substituted value is
        // unknown until the script actually runs, and for any ordinary
        // value (`k = "client_addr"`) the resulting name is a perfectly
        // ordinary, `$`-reachable one (`ns::client_addr`). Firing W215 on the
        // *name* here would flag the reconstruction, not a real defect —
        // skip that half of the check whenever the source word itself (not
        // the reconstructed text) contains an unresolved substitution.
        // The array-element `)` check below is unaffected: a
        // dynamic index round-trips verbatim (no re-bracing), so it carries
        // no such artefact.
        let name_is_static = braced
            || !self
                .source
                .get(tok.span.start() as usize..tok.span.end() as usize)
                .unwrap_or("")
                .contains(['$', '[']);

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
        if name_is_static && !is_brace_substitutable(&runtime_name, nesting) {
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
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W215,
                    site_span,
                    message,
                    Severity::Warning,
                ));
        }

        if let Some(elem) = element {
            let runtime_element: Cow<str> = if elem.contains('\\') && !braced {
                Cow::Owned(tcl_lexer::backslash_subst(elem).into_owned())
            } else {
                Cow::Borrowed(elem)
            };
            if runtime_element.contains(')') {
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        DiagCode::W215,
                        site_span,
                        "array element index contains ')'; the element can be created \
                              and read via ``set arr(idx) ...`` / ``[set \"arr(idx)\"]``, but \
                              is not reachable via $-substitution (``$arr(idx)`` reads up \
                              to the first ``)`` and stops there)"
                            .to_string(),
                        Severity::Warning,
                    ));
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
/// (namespace / proc / method) [`Scope`] children.
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

/// One collected read site: the variable word as written, where it is, and
/// whether its braces make the content a literal name (`[set {$n}]` reads the
/// cell called `$n`, not `n`).
pub(super) struct VarReadSite {
    pub name: String,
    pub span: Span,
    pub braced_literal: bool,
    substitution: bool,
}

impl VarReadSite {
    /// A read written as an ordinary `$`-substitution or expression term,
    /// whose text is already the bare name.
    fn substituted(name: &str, span: Span, braced_literal: bool) -> Self {
        Self {
            name: name.to_owned(),
            span,
            braced_literal,
            substitution: true,
        }
    }
}

/// Record the variable-name words of `cmd` that the registry marks as
/// **reads** — the `m` of `info exists m` / `array get m` / `[set m]`, and
/// the literal `$n` of `[set {$n}]`.
///
/// Original source roles from [`crate::var_refs::original_variable_name_role_words`], so no
/// command is named here: any spec declaring an [`ArgRole::VarRead`] position
/// contributes its use sites to `VarDef::references`, which is what
/// Find References / document-highlight / the minifier's rename pass consume.
/// Write roles are excluded — a write is a *declaration* site, recorded by
/// the declaring handler, not a reference.
///
/// [`ArgRole::VarRead`]: tcl_registry::ArgRole::VarRead
fn collect_name_role_reads(
    source: &str,
    analysis: &crate::analyser::AnalysisResult,
    cmd: &crate::segmenter::SegmentedCommand,
    out: &mut Vec<VarReadSite>,
) {
    for w in crate::var_refs::original_variable_name_role_words(source, analysis, cmd) {
        if w.role != tcl_registry::ArgRole::VarRead {
            continue;
        }
        out.push(VarReadSite {
            name: w.word,
            span: w.span,
            braced_literal: w.braced_literal,
            substitution: false,
        });
    }
}

/// Collect `$var` reads inside a command-substitution token,
/// recursing into nested substitutions, expr arguments, and
/// same-scope (plain) body arguments.  Structural-body arguments
/// (proc / namespace / oo) introduce a new scope and are skipped.
fn collect_cmd_subst_reads(
    source: &str,
    analysis: &crate::analyser::AnalysisResult,
    cmd_tok: Token,
    registry: &tcl_registry::CommandRegistry,
    lexer_config: tcl_lexer::LexerConfig,
    out: &mut Vec<VarReadSite>,
) {
    let Some((inner, base)) = inner_of(source, cmd_tok) else {
        return;
    };
    for cmd in crate::segmenter::segment_commands_with_offset_and_config(
        inner,
        base,
        lexer_config.nested(),
    ) {
        collect_script_command_reads(source, analysis, &cmd, registry, lexer_config, out);
    }
}

/// Record reads for one command appearing inside a substitution:
/// top-level `$var` tokens, plus recursion into nested
/// substitutions / expr args / plain bodies.
fn collect_script_command_reads(
    source: &str,
    analysis: &crate::analyser::AnalysisResult,
    cmd: &crate::segmenter::SegmentedCommand,
    registry: &tcl_registry::CommandRegistry,
    lexer_config: tcl_lexer::LexerConfig,
    out: &mut Vec<VarReadSite>,
) {
    let head_span = cmd.argv.first().map(|t| t.span);
    for tok in &cmd.all_tokens {
        if tok.kind == tcl_lexer::TokenType::Var && Some(tok.span) != head_span {
            if let Some((name, braced)) =
                crate::var_refs::original_variable_reference_form(source, *tok, lexer_config)
            {
                out.push(VarReadSite::substituted(name, tok.span, braced));
            }
        } else if tok.kind == tcl_lexer::TokenType::Cmd {
            collect_cmd_subst_reads(source, analysis, *tok, registry, lexer_config, out);
        }
    }
    // The substituted command's own `VarRead`-role name words — `puts [set
    // m]` and `puts [info exists m]` read `m`, which nothing else here sees.
    collect_name_role_reads(source, analysis, cmd, out);
    let selected =
        crate::registry_invocation::source_structure::source_registry_words(source, analysis, cmd);
    let roles = selected.as_ref().map_or_default(
        crate::registry_invocation::source_structure::OriginalRegistryWords::written_argument_roles,
    );
    let expr_idx: std::collections::HashSet<_> = roles
        .iter()
        .filter_map(|(index, role)| (*role == tcl_registry::ArgRole::Expr).then_some(*index))
        .collect();
    let body_idx: std::collections::HashSet<_> = roles
        .iter()
        .filter_map(|(index, role)| (*role == tcl_registry::ArgRole::Body).then_some(*index))
        .collect();
    let plain_body = analysis.resolved_input.as_ref().and_then(|input| {
        selected
            .as_ref()?
            .with_source_schema(&input.context_registry(), |invocation| {
                invocation.semantics.body_kind != tcl_registry::BodyKind::Structural
            })
    }) == Some(true);
    for (i, arg) in cmd.argv.iter().enumerate().skip(1) {
        if arg.kind != tcl_lexer::TokenType::Str {
            continue;
        }
        let pidx = i - 1;
        if expr_idx.contains(&pidx) {
            collect_expr_reads(source, analysis, *arg, registry, lexer_config, out);
        } else if body_idx.contains(&pidx) && plain_body {
            // Plain body inside a substitution — same scope, and the
            // main walk never reached it.
            if let Some((inner, base)) = inner_of(source, *arg) {
                for sub in crate::segmenter::segment_commands_with_offset_and_config(
                    inner,
                    base,
                    lexer_config.nested(),
                ) {
                    collect_script_command_reads(
                        source,
                        analysis,
                        &sub,
                        registry,
                        lexer_config,
                        out,
                    );
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
    analysis: &crate::analyser::AnalysisResult,
    expr_tok: Token,
    registry: &tcl_registry::CommandRegistry,
    lexer_config: tcl_lexer::LexerConfig,
    out: &mut Vec<VarReadSite>,
) {
    let Some((inner, base)) = inner_of(source, expr_tok) else {
        return;
    };
    let Ok(tokens) = tcl_lexer::Lexer::with_config(inner, lexer_config.nested()).tokenise_all()
    else {
        return;
    };
    for tok in tokens {
        let abs = Span::new(tok.span.start() + base, tok.span.end() + base);
        match tok.kind {
            tcl_lexer::TokenType::Var => {
                if let Some((name, braced)) = crate::var_refs::original_variable_reference_form(
                    source,
                    Token { span: abs, ..tok },
                    lexer_config,
                ) {
                    out.push(VarReadSite::substituted(name, abs, braced));
                }
            }
            tcl_lexer::TokenType::Cmd => {
                collect_cmd_subst_reads(
                    source,
                    analysis,
                    Token::new(tcl_lexer::TokenType::Cmd, abs),
                    registry,
                    lexer_config,
                    out,
                );
            }
            _ => {}
        }
    }
}

/// The written name of an ensemble word, for edits: a dispatch word is a whole
/// word of the ensemble invocation; a target word is a list element (or the
/// head of a prefix element) of the configuring command's option value.
fn ensemble_word_input(
    realm: &crate::realm::CommandBindingRealm,
    invocation: &crate::signature_scan::types::SignatureCommandInvocation,
    config: tcl_lexer::LexerConfig,
    rules: tcl_syntax::word_rules::WordValueRules,
    policy: tcl_syntax::naming::NamePolicyProtocol,
) -> Option<crate::signature_scan::scope::SignatureSourceNameInput> {
    use crate::signature_scan::scope::SignatureSourceNameInput;
    use crate::signature_scan::types::SignatureCommandLookup;
    match invocation.lookup {
        SignatureCommandLookup::EnsembleDispatch { invocation_offset } => realm
            .invocation_at_source("", invocation_offset)
            .original_source_name_key_at_span(invocation.range, config, rules, policy)
            .map(SignatureSourceNameInput::OriginalWord),
        SignatureCommandLookup::EnsembleTarget {
            invocation_offset,
            container_start,
            container_end,
        } => {
            let mut current = realm
                .invocation_at_source("", invocation_offset)
                .original_source_name_key_at_span(
                    Span::new(container_start, container_end),
                    config,
                    rules,
                    policy,
                )
                .map(SignatureSourceNameInput::OriginalWord)?;
            let range = invocation.range;
            for _ in 0..2 {
                let (child, extent) = current
                    .original_list_elements_with_source_spans()?
                    .into_iter()
                    .find_map(|(child, extent)| {
                        extent
                            .filter(|extent| {
                                extent.start() <= range.start() && range.end() <= extent.end()
                            })
                            .map(|extent| (child, extent))
                    })?;
                if extent == range {
                    return Some(child);
                }
                current = child;
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_positioned_reference_tests;

#[cfg(test)]
mod original_parent_purpose_tests;

#[cfg(test)]
mod original_reference_form_scope_tests;

#[cfg(test)]
mod logical_alias_occurrence_tests;

/// Source-scope relation to this analysis unit's original interpreter subject.
/// This is a readonly diagnostic boundary, not physical interpreter identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceInterpreterScopeRelation {
    /// Every retained enclosing scope stays in the analysis root's subject.
    OriginalRoot,
    /// An actual isolated child-interpreter body occurs in its ancestry.
    IsolatedBody,
}

impl super::types::AnalysisResult {
    /// Reborrow the actual source scope ancestry under its complete input.
    /// Names never encode interpreter identity here. Missing/foreign source,
    /// input or domain correspondence stays unavailable. An isolated body is
    /// a suppression boundary only, not proof that a child exists or executes.
    #[must_use]
    pub fn source_interpreter_scope_relation(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        offset: u32,
    ) -> Option<SourceInterpreterScopeRelation> {
        let input = self.resolved_input.as_ref()?;
        if input.lexer_config() != config
            || self.body_lexer_config != Some(config)
            || !self.matches_original_source_image(image, config)
            || usize::try_from(offset).ok()? >= image.bytes().len()
            || !self
                .retained_command_realm()?
                .matches_resolved_analysis_input(input)
        {
            return None;
        }
        // Isolation is a source-body boundary even where ordinary scope
        // reporting is flattened. Inspect every actual containing body rather
        // than allowing a narrower sibling to hide an isolated parent.
        let mut pending = vec![&self.global_scope];
        while let Some(scope) = pending.pop() {
            pending.extend(&scope.children);
            if !std::ptr::eq(scope, &raw const self.global_scope)
                && !scope
                    .body_span
                    .is_some_and(|span| span.start() <= offset && offset < span.end())
            {
                continue;
            }
            if scope.is_independent_interpreter_source_body() {
                return Some(SourceInterpreterScopeRelation::IsolatedBody);
            }
            match (
                &self.global_scope.original_interpreter_source_domain,
                &scope.original_interpreter_source_domain,
            ) {
                (None, None) => {}
                (Some(root), Some(current)) if std::sync::Arc::ptr_eq(root, current) => {}
                _ => return None,
            }
        }
        Some(SourceInterpreterScopeRelation::OriginalRoot)
    }
}

#[cfg(test)]
mod original_interpreter_scope_relation_controls;
