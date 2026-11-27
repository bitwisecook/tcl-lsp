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

//! Occurrences across documents: namespaces, namespace variables, aliases and method selectors.

use super::WorkspaceIndex;
use super::records::{WorkspaceNamespaceRef, WorkspaceVariableRef};
use tcl_lexer::Span;
use tcl_syntax::naming::{key_holder_and_tail, unroot_rooted_key};

impl WorkspaceIndex {
    /// Every document holding an indexed symbol declared **directly in**
    /// `namespace` — a proc, a class, or a namespace variable whose parent
    /// namespace is exactly it.
    ///
    /// One of the three candidate sources a namespace-variable *rename* must
    /// visit, and the one that catches a document whose only stake in the
    /// cell is an unqualified alias written *inside* the namespace
    /// (`namespace eval ns { proc p {} { variable v; puts $v } }`): that
    /// binding is proc-scope and deliberately not in either variable table,
    /// but the enclosing `proc` is indexed as `::ns::p`.
    ///
    /// It is **not** sufficient on its own.  An alias can be written from any
    /// namespace — a global `proc p {} { namespace upvar ::ns v local; … }`
    /// declares nothing in `::ns` at all — which is what
    /// [`Self::documents_aliasing_variable`] answers.  Both are needed;
    /// neither subsumes the other.
    #[must_use]
    pub fn documents_in_namespace(&self, namespace: &str) -> Vec<String> {
        let target = unroot_rooted_key(namespace).unwrap_or(namespace);
        let parent_matches = |qualified: &str| -> bool {
            let (holder, _) = key_holder_and_tail(qualified);
            unroot_rooted_key(holder).unwrap_or(holder) == target
        };
        let mut uris: Vec<String> = self
            .procs()
            .filter(|p| parent_matches(&p.qualified_name))
            .map(|p| p.uri.clone())
            .chain(
                self.classes()
                    .filter(|c| parent_matches(&c.qualified_name))
                    .map(|c| c.uri.clone()),
            )
            .chain(
                self.variables()
                    .filter(|v| parent_matches(&v.qualified_name))
                    .map(|v| v.uri.clone()),
            )
            .collect();
        uris.sort();
        uris.dedup();
        uris
    }

    /// Documents with an original conditional alias target matching one checked
    /// C compatibility coordinate. This selects source candidates only, never
    /// an entered link or a complete runtime target inventory.
    #[must_use]
    pub fn documents_aliasing_variable(&self, qualified_name: &str) -> Vec<String> {
        let target = unroot_rooted_key(qualified_name).unwrap_or(qualified_name);
        let mut uris: Vec<String> = self
            .variable_aliases()
            .filter(|alias| {
                alias
                    .source
                    .target_symbol()
                    .checked_c_namespace_key()
                    .is_some_and(|key| unroot_rooted_key(&key).unwrap_or(&key) == target)
            })
            .map(|alias| alias.uri.clone())
            .collect();
        uris.sort();
        uris.dedup();
        uris
    }

    /// Documents without complete retained alias-source coverage remain a
    /// barrier for every target. Missing typed receipts cannot establish that
    /// a computed or unavailable alias does not affect a proposed rename.
    #[must_use]
    pub fn documents_with_ambiguous_alias_of(&self, _qualified_name: &str) -> Vec<String> {
        let mut uris: Vec<String> = self
            .slots
            .values()
            .filter_map(|slot| self.docs.get(*slot))
            .filter(|document| !document.alias_source_coverage_complete)
            .map(|document| document.uri.clone())
            .collect();
        uris.sort();
        uris.dedup();
        uris
    }

    /// Original selectors targeting one canonical document-owned method.
    /// Declaration advice and temporal receiver entries retain their separate
    /// purpose; no reporting-name family or native allocation is reconstructed.
    #[must_use]
    pub fn original_method_reference_rows(
        &self,
        target: &crate::method_symbol::OriginalMethodCandidate,
        include_declaration: bool,
    ) -> Vec<(String, Span, tcl_lexer::SourceImage, tcl_lexer::LexerConfig)> {
        let mut spans = Vec::new();
        if include_declaration {
            spans.push((
                target.uri().to_owned(),
                target.declaration_span(),
                target.declaration_image().clone(),
                target.declaration_config(),
            ));
        }
        for document in &self.docs {
            for query in &document.original_method_queries {
                if let std::ops::ControlFlow::Break(Some(candidate)) =
                    crate::method_symbol::candidate(self, &document.uri, query)
                    && candidate.same_declaration(target)
                {
                    spans.push((
                        document.uri.clone(),
                        query.span(),
                        query.consumer_image().clone(),
                        query.consumer_config(),
                    ));
                }
            }
        }
        spans.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| (left.1.start(), left.1.end()).cmp(&(right.1.start(), right.1.end())))
        });
        spans.dedup();
        spans
    }

    /// Every indexed namespace-name occurrence.
    pub fn namespace_refs(&self) -> impl Iterator<Item = &WorkspaceNamespaceRef> {
        self.docs.iter().flat_map(|doc| doc.namespace_refs.iter())
    }

    /// Original variable naming rows joined by the shared byte-table symbol.
    /// These are source occurrences; loading, alias lifetime and native frame
    /// entry remain independent. Display maps supply no lookup fallback.
    pub fn original_variable_occurrences<'a>(
        &'a self,
        symbol: &'a tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
        exclude_uri: &'a str,
    ) -> impl Iterator<
        Item = (
            &'a str,
            &'a tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableOccurrence,
        ),
    > + 'a {
        let mut seen = std::collections::HashSet::new();
        self.docs
            .iter()
            .filter(move |document| document.uri != exclude_uri)
            .flat_map(move |document| {
                document
                    .original_variables
                    .iter()
                    .filter(move |occurrence| {
                        symbol.is_namespace() && occurrence.symbol() == symbol
                    })
                    .map(move |occurrence| (document.uri.as_str(), occurrence))
            })
            .filter(move |(owner, occurrence)| {
                seen.insert((*owner, occurrence.span(), occurrence.is_declaration()))
            })
    }

    /// Exact namespace occurrences from independently retained scope and name
    /// policy. Document ownership and source words remain in each row.
    pub fn original_namespace_occurrences<'a>(
        &'a self,
        symbol: &'a crate::namespace_symbol::OriginalNamespaceSymbol,
        exclude_uri: &'a str,
    ) -> impl Iterator<Item = &'a WorkspaceNamespaceRef> {
        let mut seen = std::collections::HashSet::new();
        self.namespace_refs().filter(move |row| {
            row.uri != exclude_uri
                && row.source.source_namespace.as_ref() == Some(symbol.scope())
                && row.source.name_policy == Some(symbol.policy())
                && row
                    .source
                    .original_name_input
                    .as_ref()
                    .is_some_and(|input| input.policy() == symbol.policy())
                && seen.insert((row.uri.as_str(), row.span, row.declares))
        })
    }

    /// Actual declaring rows whose retained components include this namespace
    /// as a strict parent. This grants no substring source edit.
    pub fn original_namespace_descendant_declarations<'a>(
        &'a self,
        symbol: &'a crate::namespace_symbol::OriginalNamespaceSymbol,
        exclude_uri: &'a str,
    ) -> impl Iterator<Item = &'a WorkspaceNamespaceRef> {
        let mut seen = std::collections::HashSet::new();
        self.namespace_refs().filter(move |row| {
            row.declares
                && row.uri != exclude_uri
                && row.source.name_policy == Some(symbol.policy())
                && row.source.source_namespace.as_ref().is_some_and(|scope| {
                    symbol.scope().is_strict_ancestor_of(scope, symbol.policy()) == Some(true)
                })
                && seen.insert((row.uri.as_str(), row.span))
        })
    }

    /// Exact retained namespace selected from all indexed source receipts.
    #[must_use]
    pub fn retained_namespace(
        &self,
        report: &str,
    ) -> Option<(
        tcl_names::namespace::SignatureNamespaceScope,
        tcl_syntax::naming::NamePolicyProtocol,
    )> {
        crate::namespace_symbol::retained_namespace_from_refs(
            self.namespace_refs().map(|row| &row.source),
            report,
        )
    }

    /// **Declaring** sites of the namespace `qualified_name` — the name word
    /// of each `namespace eval` block that creates or extends it — excluding
    /// any in `exclude_uri` (pass `""` to exclude nothing).
    ///
    /// A set, not an `Option`, and for a stronger reason than the variable
    /// tier's: reopening a namespace is the *normal* way to build one, and
    /// tclsh 9.0.4 / 8.6.16 agree byte-for-byte that two `namespace eval ::a
    /// {}` blocks are one namespace (`info vars ::a::*` shows both blocks'
    /// variables).  Every block is a real definition site.
    ///
    /// Exact-name matching only: the analyser already rooted every relative
    /// spelling, so there is no candidate list to walk.
    #[must_use]
    pub fn namespace_declarations_qualified<'a>(
        &'a self,
        qualified_name: &str,
        exclude_uri: &str,
    ) -> Vec<&'a WorkspaceNamespaceRef> {
        let Some((scope, policy)) = self.retained_namespace(qualified_name) else {
            return Vec::new();
        };
        self.namespace_refs()
            .filter(|n| n.declares && n.uri != exclude_uri)
            .filter(|n| {
                n.source.source_namespace.as_ref() == Some(&scope)
                    && n.source.name_policy == Some(policy)
            })
            .collect()
    }

    /// Declaring sites whose namespace is a **strict descendant** of
    /// `qualified_name` — the rows that create it *implicitly*, as a parent
    /// (`namespace eval ::p::q::r {}` really creates `::p::q`), excluding any
    /// in `exclude_uri`.
    ///
    /// The cross-document half of the implicit-parent rule: the in-document
    /// tier answers implicit parents from its own `namespace_refs`, and
    /// without this query a namespace whose only creating block lives in a
    /// sibling file has no answer at all.
    ///
    /// Rows only — the *span* an implicit answer reports is the covering
    /// prefix of the written word, a sub-range that needs the declaring
    /// document's text, so the caller pairs each row with its source through
    /// [`crate::namespace_symbol::namespace_implicit_parent_span_in`].
    #[must_use]
    pub fn namespace_declarations_under<'a>(
        &'a self,
        qualified_name: &str,
        exclude_uri: &str,
    ) -> Vec<&'a WorkspaceNamespaceRef> {
        let Some((scope, policy)) = self.retained_namespace(qualified_name) else {
            return Vec::new();
        };
        self.namespace_refs()
            .filter(|n| n.declares && n.uri != exclude_uri)
            .filter(|n| {
                n.source.name_policy == Some(policy)
                    && n.source.source_namespace.as_ref() != Some(&scope)
            })
            .filter(|n| {
                crate::namespace_symbol::retained_namespace_from_refs(
                    std::iter::once(&n.source),
                    qualified_name,
                )
                .as_ref()
                    == Some(&(scope.clone(), policy))
            })
            .collect()
    }

    /// Non-declaring occurrences of the namespace `qualified_name`, excluding
    /// any in `exclude_uri` — the reference-side twin of
    /// [`Self::namespace_declarations_qualified`].
    #[must_use]
    pub fn namespace_refs_of<'a>(
        &'a self,
        qualified_name: &str,
        exclude_uri: &str,
    ) -> Vec<&'a WorkspaceNamespaceRef> {
        let Some((scope, policy)) = self.retained_namespace(qualified_name) else {
            return Vec::new();
        };
        self.namespace_refs()
            .filter(|n| !n.declares && n.uri != exclude_uri)
            .filter(|n| {
                n.source.source_namespace.as_ref() == Some(&scope)
                    && n.source.name_policy == Some(policy)
            })
            .collect()
    }

    /// Occurrence (read / write) sites naming the namespace variable
    /// `qualified_name`, excluding any in `exclude_uri`.  The reference-side
    /// twin of [`Self::variable_definitions_qualified`], matched the same
    /// exact way.
    #[must_use]
    pub fn variable_refs_of<'a>(
        &'a self,
        qualified_name: &str,
        exclude_uri: &str,
    ) -> Vec<&'a WorkspaceVariableRef> {
        let target = unroot_rooted_key(qualified_name).unwrap_or(qualified_name);
        self.variable_refs()
            .filter(|v| v.uri != exclude_uri)
            .filter(|v| unroot_rooted_key(&v.qualified_name).unwrap_or(&v.qualified_name) == target)
            .collect()
    }
}
