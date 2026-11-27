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

//! Namespace-qualified variables: lookup by qualified name, alias links, and the qualified references recorded and attached after the walk.

use super::super::state::Analyser;
use super::super::types::{Scope, ScopeKind, VarDef};
use super::command_namespaces::advance_command_resolution_namespace;
use super::scope_at;
use std::collections::HashMap;
use tcl_lexer::Span;

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
    /// Record a **namespace-qualified** variable occurrence (see
    /// [`super::super::types::QualifiedVarRef`]) so the workspace index can build a
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
    pub(super) fn record_qualified_var_ref(
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
        original_namespace: Option<tcl_names::namespace::SignatureNamespaceScope>,
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
            .push(super::super::types::QualifiedVarRef {
                qualified_name: qualified,
                span,
                original_name_input,
                original_namespace,
            });
    }

    /// Post-walk pass: attach each recorded [`super::super::types::QualifiedVarRef`]
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
    pub(in super::super) fn attach_qualified_var_references(&mut self) {
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
}
