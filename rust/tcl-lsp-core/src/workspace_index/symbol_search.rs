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

//! Workspace symbols matching a query.

use super::class_dispatch::{RetractionIndex, effective_member};
use super::{DocumentRecords, WorkspaceIndex};
use crate::workspace_symbols::{
    IndexedWorkspaceSymbol, WorkspaceSymbolKind, matches_query, namespace_of,
};
use std::collections::HashSet;

impl DocumentRecords {
    /// Append this document's [`IndexedWorkspaceSymbol`]s matching
    /// `lower_query` to `out`, stopping once `out` reaches `limit`.
    ///
    /// The order — procs, then classes with their members, then registry
    /// symbol-definer definitions — mirrors the per-document outline the
    /// document-symbol provider produces, and each table is already in source
    /// order (see [`sorted_by_span`]).
    fn collect_symbols_matching(
        &self,
        lower_query: &str,
        limit: usize,
        retractions: &RetractionIndex<'_>,
        out: &mut Vec<IndexedWorkspaceSymbol>,
    ) {
        let Some(context) = self.diagnostic_source.as_ref() else {
            return;
        };
        if context.uses_original_names() {
            self.collect_original_symbols_matching(lower_query, limit, out);
            return;
        }
        for proc_def in &self.procs {
            if out.len() >= limit {
                return;
            }
            if matches_query(&proc_def.name, lower_query)
                || matches_query(&proc_def.qualified_name, lower_query)
            {
                out.push(IndexedWorkspaceSymbol {
                    uri: proc_def.uri.clone(),
                    name: proc_def.name.clone(),
                    container_name: namespace_of(&proc_def.qualified_name),
                    kind: WorkspaceSymbolKind::Function,
                    name_span: proc_def.name_span,
                    original_location: None,
                    logical_source: None,
                });
            }
        }
        // A constructor's only name is the keyword, so whether the query
        // admits one is decided once rather than per class.
        let wants_constructors = matches_query("constructor", lower_query);
        for class_def in &self.classes {
            if out.len() >= limit {
                return;
            }
            if matches_query(&class_def.name, lower_query)
                || matches_query(&class_def.qualified_name, lower_query)
            {
                out.push(IndexedWorkspaceSymbol {
                    uri: class_def.uri.clone(),
                    name: class_def.name.clone(),
                    container_name: namespace_of(&class_def.qualified_name),
                    kind: WorkspaceSymbolKind::Class,
                    name_span: class_def.name_span,
                    original_location: None,
                    logical_source: None,
                });
            }
            // Members carry the class's qualified name as their container, so
            // an editor renders them as `ClassName::methodName`.
            //
            // Enumerated through the workspace's member fold, not straight off
            // this record's own table: a member a cross-file `oo::define ::C {
            // renamemethod old new }` moved is declared here under `old` and
            // dispatches as `new`, and one deleted the same way is not a
            // member at all.  An arrived member's location is
            // the arrival word in the retracting document, which is also what
            // go-to-definition on the new name answers with.
            for method in &class_def.methods {
                if out.len() >= limit {
                    return;
                }
                let Some(em) = effective_member(retractions, class_def, method) else {
                    continue;
                };
                if matches_query(em.name, lower_query) {
                    out.push(IndexedWorkspaceSymbol {
                        uri: em.name_uri.to_owned(),
                        name: em.name.to_owned(),
                        container_name: Some(class_def.qualified_name.clone()),
                        kind: WorkspaceSymbolKind::Method,
                        name_span: em.name_span,
                        original_location: None,
                        logical_source: None,
                    });
                }
            }
            if wants_constructors {
                for &name_span in &class_def.constructor_spans {
                    if out.len() >= limit {
                        return;
                    }
                    out.push(IndexedWorkspaceSymbol {
                        uri: class_def.uri.clone(),
                        name: "constructor".to_owned(),
                        container_name: Some(class_def.qualified_name.clone()),
                        kind: WorkspaceSymbolKind::Constructor,
                        name_span,
                        original_location: None,
                        logical_source: None,
                    });
                }
            }
        }
        for sym in &self.defined_symbols {
            if out.len() >= limit {
                return;
            }
            if matches_query(&sym.name, lower_query)
                || matches_query(&sym.qualified_name, lower_query)
            {
                out.push(IndexedWorkspaceSymbol {
                    uri: sym.uri.clone(),
                    name: sym.name.clone(),
                    container_name: namespace_of(&sym.qualified_name),
                    kind: WorkspaceSymbolKind::from(sym.kind),
                    name_span: sym.name_span,
                    original_location: None,
                    logical_source: None,
                });
            }
        }
    }

    fn collect_original_symbols_matching(
        &self,
        lower_query: &str,
        limit: usize,
        out: &mut Vec<IndexedWorkspaceSymbol>,
    ) {
        for symbol in &self.original_symbols {
            if out.len() >= limit {
                return;
            }
            if matches_query(&symbol.name, lower_query)
                || symbol.container_name.as_ref().is_some_and(|container| {
                    matches_query(
                        &tcl_syntax::naming::rooted_member_key(container, &symbol.name),
                        lower_query,
                    )
                })
            {
                out.push(symbol.clone());
            }
        }
    }
}

impl WorkspaceIndex {
    /// The workspace's symbols matching `query`, at most `limit` of them —
    /// the whole `workspace/symbol` answer.
    ///
    /// Every indexed document is searched, not only the ones the editor has
    /// open, so a symbol in a file the user has never opened is reachable from
    /// the picker.  The index is refreshed on each document's diagnostics
    /// publish (~50 ms after an edit), which is the freshness a symbol picker
    /// gets: a name typed a moment ago appears once that publish lands.  It is
    /// the same staleness every other cross-document feature answers with, and
    /// the alternative — re-analysing every open buffer per keystroke in the
    /// Ctrl+T box — costs far more than the freshness is worth.
    ///
    /// The scan is **document-major**: each document contributes its procs,
    /// then its classes (with their methods and constructors), then its
    /// registry symbol-definer definitions, before the next document is
    /// looked at.  So a `limit`-truncated answer is a prefix of the workspace
    /// rather than a prefix of one symbol table — a capped result still holds
    /// classes and test cases, not only procs — and results arrive grouped by
    /// URI, which lets the caller resolve each document's source once.
    #[must_use]
    pub fn symbols_matching(&self, query: &str, limit: usize) -> Vec<IndexedWorkspaceSymbol> {
        self.symbols_matching_excluding(query, limit, std::iter::empty::<&str>())
    }

    /// The workspace's symbols matching `query`, excluding every document URI
    /// in `excluded_uris` before applying `limit`.
    ///
    /// Filtering at the document scan keeps the result bounded without letting
    /// excluded documents consume the caller's capacity. The server uses this
    /// while a watched deletion has marked an open buffer orphaned but its
    /// atomic index-removal transaction is still waiting.
    #[must_use]
    pub fn symbols_matching_excluding<'a>(
        &self,
        query: &str,
        limit: usize,
        excluded_uris: impl IntoIterator<Item = &'a str>,
    ) -> Vec<IndexedWorkspaceSymbol> {
        let lower_query = query.to_lowercase();
        let mut out: Vec<IndexedWorkspaceSymbol> = Vec::new();
        let excluded_slots: HashSet<usize> = excluded_uris
            .into_iter()
            .filter_map(|uri| self.slots.get(uri).copied())
            .collect();
        // Built once for the whole scan — the class-member walk needs the
        // workspace's cross-document retractions, which no single document's
        // records can answer for themselves.
        let retractions = self.retraction_index_excluding(&excluded_slots);
        for (slot, doc) in self.docs.iter().enumerate() {
            if out.len() >= limit {
                break;
            }
            if excluded_slots.contains(&slot) {
                continue;
            }
            doc.collect_symbols_matching(&lower_query, limit, &retractions, &mut out);
        }
        // Logical compatibility rows are report candidates, not source facts.
        // Reattach the actual target document's retained source/config/Registry
        // context so the server can map their original spans only against that
        // exact image. Cross-document effective-member rows use the arrival
        // document's own context, not the class record that contributed them.
        for symbol in &mut out {
            if symbol.original_location.is_some() {
                continue;
            }
            let Some(slot) = self.slots.get(&symbol.uri) else {
                continue;
            };
            let Some(document) = self.docs.get(*slot) else {
                continue;
            };
            symbol.logical_source = document.diagnostic_source.as_ref().and_then(|context| {
                crate::workspace_symbols::LogicalWorkspaceSymbolSource::capture(
                    symbol.uri.clone(),
                    context,
                    symbol.name_span,
                    document.revision,
                )
            });
        }
        out
    }
}
