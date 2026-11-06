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

//! Workspace-symbol facts borrow their actual published interpretation.

use super::super::super::{DocumentPathAdmission, DocumentState};
use super::super::{ReadonlySourceState, WorkspaceSymbolRow};
use super::{
    Arc, Backend, OriginalSourceViewAttempt, OriginalSourceViewTable, PublishedOriginalSourceView,
};
use tcl_lsp_core::workspace_symbols::IndexedWorkspaceSymbol;

/// Ownership captured from the current index/table, not a lasting lease.
/// The final source transaction rejoins every owner and ancestor atomically.
pub(crate) struct WorkspaceSymbolSourceView {
    hit: IndexedWorkspaceSymbol,
    scope: Arc<WorkspaceSymbolViewScope>,
}

struct WorkspaceSymbolViewScope {
    view: Arc<PublishedOriginalSourceView>,
    sources: Vec<Arc<PublishedOriginalSourceView>>,
    saved: Vec<(Arc<DocumentPathAdmission>, Arc<[u8]>)>,
}

/// Query-local ownership cache; no captured lease can outlive the final join.
#[derive(Default)]
pub(crate) struct WorkspaceSymbolSourceViewCache {
    scopes: std::collections::HashMap<super::OriginalSourceViewId, Arc<WorkspaceSymbolViewScope>>,
}

impl WorkspaceSymbolSourceView {
    pub(in crate::readonly_reference_reports) fn admissions(
        &self,
    ) -> impl Iterator<Item = &DocumentPathAdmission> {
        self.scope
            .sources
            .iter()
            .map(|view| view.draft.physical.admission.as_ref())
    }

    pub(in crate::readonly_reference_reports) fn saved_sources(
        &self,
    ) -> impl Iterator<Item = (&DocumentPathAdmission, &[u8])> {
        self.scope
            .saved
            .iter()
            .map(|(admission, bytes)| (admission.as_ref(), bytes.as_ref()))
    }

    fn matches_row(&self, row: &WorkspaceSymbolRow) -> bool {
        let snapshot = &row.2;
        self.scope.view.draft.physical.uri == row.1
            && self.scope.view.draft.physical.uri.as_str() == self.hit.uri
            && self.scope.view.draft.physical.source == snapshot.text
            && self.hit.original_location.as_ref().is_some_and(|owner| {
                owner.uri() == self.hit.uri
                    && owner.validated_span(
                        &snapshot.text,
                        &self.scope.view.draft.producer.seed.analysis,
                    ) == Some(self.hit.name_span)
            })
            && row.0
                == Backend::workspace_symbol_information(
                    self.hit.name.clone(),
                    self.hit.kind,
                    self.hit.container_name.clone(),
                    self.hit.name_span,
                    &row.1,
                    snapshot,
                )
    }
}

/// Held before the index guard and through the same provider lease/projection.
pub(in crate::readonly_reference_reports) struct WorkspaceSymbolViewGuards<'a> {
    _rehoming: Option<tokio::sync::MutexGuard<'a, ()>>,
    table: Option<std::sync::MutexGuard<'a, OriginalSourceViewTable>>,
}

impl WorkspaceSymbolViewGuards<'_> {
    pub(in crate::readonly_reference_reports) fn matches(
        &self,
        proof: &WorkspaceSymbolSourceView,
        row: &WorkspaceSymbolRow,
        state: &ReadonlySourceState<'_>,
        index: &tcl_lsp_core::workspace_index::WorkspaceIndex,
        epochs: (u64, u64),
    ) -> bool {
        let Some(table) = &self.table else {
            return false;
        };
        proof.matches_row(row)
            && proof.scope.sources.iter().all(|view| {
                view.draft.matches_physical_state(state, epochs.0, epochs.1)
                    && table.contains_actual(view)
                    && table.parent_chain_is_live(&view.draft)
                    && view.is_indexed(index)
            })
    }
}

impl Backend {
    /// Select only an actual indexed interpretation that owns this exact fact.
    /// Missing or ambiguous view ownership cannot borrow a standalone analysis.
    pub(crate) fn capture_workspace_symbol_source_view(
        &self,
        hit: &IndexedWorkspaceSymbol,
        snapshot: &DocumentState,
        cache: &mut WorkspaceSymbolSourceViewCache,
    ) -> OriginalSourceViewAttempt<Option<WorkspaceSymbolSourceView>> {
        let view = match self.workspace_symbol_view_owner(hit, snapshot) {
            OriginalSourceViewAttempt::Ready(Some(view)) => view,
            OriginalSourceViewAttempt::Ready(None) => {
                return OriginalSourceViewAttempt::Ready(None);
            }
            OriginalSourceViewAttempt::RetryCapture => {
                return OriginalSourceViewAttempt::RetryCapture;
            }
            OriginalSourceViewAttempt::Unavailable => {
                return OriginalSourceViewAttempt::Unavailable;
            }
        };
        if let Some(scope) = cache.scopes.get(&view.id)
            && Arc::ptr_eq(&scope.view, &view)
        {
            return OriginalSourceViewAttempt::Ready(Some(WorkspaceSymbolSourceView {
                hit: hit.clone(),
                scope: Arc::clone(scope),
            }));
        }
        let Some(ancestors) = view.draft.ancestors() else {
            return OriginalSourceViewAttempt::Unavailable;
        };
        let sources = std::iter::once(Arc::clone(&view))
            .chain(ancestors)
            .collect::<Vec<_>>();
        let Some(saved) = Self::workspace_symbol_original_saved_sources(&sources) else {
            return OriginalSourceViewAttempt::Unavailable;
        };
        let scope = Arc::new(WorkspaceSymbolViewScope {
            view,
            sources,
            saved,
        });
        cache.scopes.insert(scope.view.id, Arc::clone(&scope));
        OriginalSourceViewAttempt::Ready(Some(WorkspaceSymbolSourceView {
            hit: hit.clone(),
            scope,
        }))
    }

    fn workspace_symbol_original_saved_sources(
        sources: &[Arc<PublishedOriginalSourceView>],
    ) -> Option<Vec<(Arc<DocumentPathAdmission>, Arc<[u8]>)>> {
        let mut saved = Vec::new();
        for view in sources {
            let physical = &view.draft.physical;
            let publication = &physical.publication;
            if publication.open_revision.is_some() {
                continue;
            }
            // Preserve the bytes and mapping captured by this actual producer.
            // A newer read with equal normalized text cannot renew its owner.
            let admission = publication.document.path_admission.as_ref()?;
            let bytes = publication.document.saved_source_bytes.as_ref()?;
            if admission.as_ref() != physical.admission.as_ref() || admission.uri() != &physical.uri
            {
                return None;
            }
            saved.push((Arc::clone(admission), Arc::clone(bytes)));
        }
        Some(saved)
    }

    fn workspace_symbol_view_owner(
        &self,
        hit: &IndexedWorkspaceSymbol,
        snapshot: &DocumentState,
    ) -> OriginalSourceViewAttempt<Option<Arc<PublishedOriginalSourceView>>> {
        let Ok(table) = self.original_source_views.try_lock() else {
            return OriginalSourceViewAttempt::RetryCapture;
        };
        let Ok(index) = self.workspace_index.try_read() else {
            return OriginalSourceViewAttempt::RetryCapture;
        };
        let indexed = index.original_source_views(&hit.uri);
        if indexed.is_empty() {
            return OriginalSourceViewAttempt::Ready(None);
        }
        let Some(owner) = hit.original_location.as_ref() else {
            return OriginalSourceViewAttempt::Ready(None);
        };
        let Ok(uri) = hit.uri.parse::<tower_lsp_server::ls_types::Uri>() else {
            return OriginalSourceViewAttempt::Unavailable;
        };
        let mut matches = table
            .by_uri
            .get(&uri)
            .into_iter()
            .flatten()
            .filter_map(|id| table.by_id.get(id))
            .filter(|view| {
                view.draft.physical.uri.as_str() == hit.uri
                    && view.draft.physical.source == snapshot.text
                    && table.contains_actual(view)
                    && view.is_indexed(&index)
                    && owner.uri() == hit.uri
                    && owner.validated_span(&snapshot.text, &view.draft.producer.seed.analysis)
                        == Some(hit.name_span)
            });
        let Some(view) = matches.next() else {
            return OriginalSourceViewAttempt::Unavailable;
        };
        if matches.next().is_some() {
            return OriginalSourceViewAttempt::Unavailable;
        }
        OriginalSourceViewAttempt::Ready(Some(Arc::clone(view)))
    }

    pub(in crate::readonly_reference_reports) fn workspace_symbol_view_guards(
        &self,
        views: &[Option<WorkspaceSymbolSourceView>],
    ) -> Option<WorkspaceSymbolViewGuards<'_>> {
        if views.iter().all(Option::is_none) {
            return Some(WorkspaceSymbolViewGuards {
                _rehoming: None,
                table: None,
            });
        }
        let rehoming = self.rehoming_gate.try_lock().ok()?;
        let table = self.original_source_views.try_lock().ok()?;
        Some(WorkspaceSymbolViewGuards {
            _rehoming: Some(rehoming),
            table: Some(table),
        })
    }
}

#[cfg(test)]
mod tests;
