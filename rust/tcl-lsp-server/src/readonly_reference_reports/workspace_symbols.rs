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

//! Workspace-symbol responses: the rows that are current, their source inputs and the transaction the response is projected under.

use super::super::{Backend, DocumentPathAdmission, DocumentSourceResult};
use super::snapshot_currency::open_document_advanced;
use super::{
    MovedIndexRows, ReadonlyPublicationInputs, ReadonlySourceState, ReadonlyTransaction,
    WorkspaceSymbolPublication, WorkspaceSymbolRow, WorkspaceSymbolSourceAnalysis, source_views,
};
use tower_lsp_server::ls_types::Uri;

impl WorkspaceSymbolPublication<'_> {
    pub(super) fn current_rows(
        &self,
        state: &ReadonlySourceState<'_>,
        index: &tcl_lsp_core::workspace_index::WorkspaceIndex,
        view_guards: &source_views::WorkspaceSymbolViewGuards<'_>,
    ) -> Vec<usize> {
        let owners = self
            .rows
            .iter()
            .map(|(_, uri, _, _)| uri.as_str())
            .collect();
        let current_inputs = Backend::workspace_symbol_current_inputs(
            &owners,
            self.analyses,
            state,
            self.epochs,
            self.diagnostic,
        );
        let mut current = Vec::with_capacity(self.rows.len());
        for (ordinal, (_, uri, snapshot, revision)) in self.rows.iter().enumerate() {
            if !current_inputs.contains(uri.as_str())
                || self
                    .analyses
                    .get(uri.as_str())
                    .and_then(Option::as_ref)
                    .is_none_or(|analysis| {
                        !WorkspaceSymbolSourceAnalysis::matches_snapshot(analysis, uri, snapshot)
                    })
            {
                continue;
            }
            if !workspace_symbol_index_snapshot_is_current(
                uri,
                snapshot,
                *revision,
                state.open.get(uri),
                index,
            ) {
                self.diagnostic.record("lease.index_snapshot_refused", &[]);
                self.index_moved
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                continue;
            }
            if self
                .views
                .get(ordinal)
                .and_then(Option::as_ref)
                .is_some_and(|view| {
                    !view_guards.matches(view, &self.rows[ordinal], state, index, self.epochs)
                })
            {
                self.diagnostic.record("lease.source_view_refused", &[]);
                continue;
            }
            current.push(ordinal);
        }
        current
    }

    fn source_inputs<'a>(
        &'a self,
        admissions: &[&'a DocumentPathAdmission],
        current: &[usize],
    ) -> ReadonlyPublicationInputs<'a> {
        let mut admissions = admissions.to_vec();
        let mut sources = super::super::DocumentState::saved_source_inputs(
            current.iter().map(|&ordinal| &self.rows[ordinal].2),
        );
        // Subtractive deduplication only: independent retained receipts stay
        // distinct even when their URI, path or byte values happen to agree.
        let mut selected_admissions = admissions
            .iter()
            .map(|admission| std::ptr::from_ref(*admission))
            .collect::<std::collections::HashSet<_>>();
        let mut selected_images = sources
            .iter()
            .map(|(admission, bytes)| (std::ptr::from_ref(*admission), bytes.as_ptr(), bytes.len()))
            .collect::<std::collections::HashSet<_>>();
        for view in current
            .iter()
            .filter_map(|&ordinal| self.views.get(ordinal)?.as_ref())
        {
            for admission in view.admissions() {
                if selected_admissions.insert(std::ptr::from_ref(admission)) {
                    admissions.push(admission);
                }
            }
            for (admission, bytes) in view.saved_sources() {
                if selected_images.insert((
                    std::ptr::from_ref(admission),
                    bytes.as_ptr(),
                    bytes.len(),
                )) {
                    sources.push((admission, bytes));
                }
            }
        }
        ReadonlyPublicationInputs {
            admissions: std::borrow::Cow::Owned(admissions),
            saved_sources: (!sources.is_empty()).then_some(std::borrow::Cow::Owned(sources)),
        }
    }
}

impl Backend {
    /// Project once while the request's captured snapshot remains valid: the
    /// analyser-input epoch, factory generation, configuration and provider
    /// receipts it was analysed under, and each document revision it captured.
    /// A document whose client version advanced after capture keeps its
    /// captured answer (#1854); one whose captured revision was replaced in
    /// place, closed or deleted is withdrawn, and so is a closed document
    /// whose disk read or index revision changed. This retains possible
    /// symbol advice, not a complete workspace absence census.
    #[cfg(test)]
    pub(in super::super) async fn finish_workspace_symbol_response(
        &self,
        analyses: &std::collections::HashMap<String, Option<WorkspaceSymbolSourceAnalysis>>,
        rows: &[WorkspaceSymbolRow],
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> DocumentSourceResult<Option<tower_lsp_server::ls_types::WorkspaceSymbolResponse>> {
        self.workspace_symbol_response_transaction(analyses, rows, inputs_epoch, factory_generation)
            .await
            .map(ReadonlyTransaction::published)
    }

    #[cfg(test)]
    pub(in super::super) async fn workspace_symbol_response_transaction(
        &self,
        analyses: &std::collections::HashMap<String, Option<WorkspaceSymbolSourceAnalysis>>,
        rows: &[WorkspaceSymbolRow],
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> DocumentSourceResult<
        ReadonlyTransaction<tower_lsp_server::ls_types::WorkspaceSymbolResponse>,
    > {
        self.workspace_symbol_response_transaction_diagnostic(
            analyses,
            rows,
            inputs_epoch,
            factory_generation,
            &super::super::WorkspaceSymbolDiagnostic::default(),
        )
        .await
    }

    #[cfg(test)]
    pub(in super::super) async fn workspace_symbol_response_transaction_diagnostic(
        &self,
        analyses: &std::collections::HashMap<String, Option<WorkspaceSymbolSourceAnalysis>>,
        rows: &[WorkspaceSymbolRow],
        inputs_epoch: u64,
        factory_generation: u64,
        diagnostic: &super::super::WorkspaceSymbolDiagnostic,
    ) -> DocumentSourceResult<
        ReadonlyTransaction<tower_lsp_server::ls_types::WorkspaceSymbolResponse>,
    > {
        self.workspace_symbol_response_transaction_with_views(
            analyses,
            rows,
            &[],
            (inputs_epoch, factory_generation),
            diagnostic,
            MovedIndexRows::Omit,
        )
        .await
    }

    pub(in super::super) async fn workspace_symbol_response_transaction_with_views(
        &self,
        analyses: &std::collections::HashMap<String, Option<WorkspaceSymbolSourceAnalysis>>,
        rows: &[WorkspaceSymbolRow],
        views: &[Option<source_views::WorkspaceSymbolSourceView>],
        epochs: (u64, u64),
        diagnostic: &super::super::WorkspaceSymbolDiagnostic,
        moved: MovedIndexRows,
    ) -> DocumentSourceResult<
        ReadonlyTransaction<tower_lsp_server::ls_types::WorkspaceSymbolResponse>,
    > {
        if rows.is_empty() {
            diagnostic.record("lease.publish_guard.1", &[]);
            return Ok(ReadonlyTransaction::PublishRefused);
        }
        let owners = rows
            .iter()
            .map(|(_, uri, _, _)| uri.as_str())
            .collect::<std::collections::HashSet<_>>();
        let Some(selected) = owners
            .iter()
            .map(|uri| analyses.get(*uri)?.as_ref())
            .collect::<Option<Vec<_>>>()
        else {
            diagnostic.record("lease.publish_guard.2", &[]);
            return Ok(ReadonlyTransaction::PublishRefused);
        };
        if !rows.iter().any(|(_, uri, snapshot, _)| {
            analyses
                .get(uri.as_str())
                .and_then(Option::as_ref)
                .is_some_and(|analysis| {
                    WorkspaceSymbolSourceAnalysis::matches_snapshot(analysis, uri, snapshot)
                })
        }) {
            diagnostic.record("lease.publish_guard.3", &[]);
            return Ok(ReadonlyTransaction::PublishRefused);
        }
        let Some(admissions) = self.workspace_symbol_source_admissions(&selected, rows, diagnostic)
        else {
            return Ok(ReadonlyTransaction::PublishRefused);
        };
        if !views.is_empty() && views.len() != rows.len() {
            return Ok(ReadonlyTransaction::PublishRefused);
        }
        let publication = WorkspaceSymbolPublication {
            analyses,
            rows,
            views,
            epochs,
            diagnostic,
            index_moved: std::sync::atomic::AtomicBool::new(false),
        };
        self.readonly_source_transaction_with_inputs(
            epochs,
            |(_, _, current): &(_, _, Vec<usize>)| publication.source_inputs(&admissions, current),
            diagnostic,
            |state| {
                // Keep view/table ownership before the existing index guard,
                // through one combined provider lease and response projection.
                let view_guards = self.workspace_symbol_view_guards(views)?;
                let Ok(index) = self.workspace_index.try_read() else {
                    diagnostic.record("lease.index_busy", &[]);
                    return None;
                };
                let current = publication.current_rows(state, &index, &view_guards);
                if moved == MovedIndexRows::Recapture
                    && publication
                        .index_moved
                        .load(std::sync::atomic::Ordering::Relaxed)
                {
                    return None;
                }
                if current.is_empty() {
                    diagnostic.record("lease.no_current_rows", &[]);
                }
                (!current.is_empty()).then_some((view_guards, index, current))
            },
            |_, (_, _, current)| {
                Some(tower_lsp_server::ls_types::WorkspaceSymbolResponse::Flat(
                    current
                        .iter()
                        .map(|&ordinal| rows[ordinal].0.clone())
                        .collect(),
                ))
            },
        )
        .await
    }

    fn workspace_symbol_current_inputs<'a>(
        owners: &std::collections::HashSet<&'a str>,
        analyses: &std::collections::HashMap<String, Option<WorkspaceSymbolSourceAnalysis>>,
        state: &ReadonlySourceState<'_>,
        epochs: (u64, u64),
        diagnostic: &super::super::WorkspaceSymbolDiagnostic,
    ) -> std::collections::HashSet<&'a str> {
        let (inputs_epoch, factory_generation) = epochs;
        let current_inputs = owners
            .iter()
            .copied()
            .filter(|uri| {
                analyses
                    .get(*uri)
                    .and_then(Option::as_ref)
                    .is_some_and(|analysis| {
                        WorkspaceSymbolSourceAnalysis::matches_request_snapshot(
                            analysis,
                            state,
                            inputs_epoch,
                            factory_generation,
                            diagnostic,
                        )
                    })
            })
            .collect::<std::collections::HashSet<_>>();
        diagnostic.record(
            "lease.inputs_current",
            &[current_inputs.len(), owners.len()],
        );
        current_inputs
    }

    fn workspace_symbol_source_admissions<'a>(
        &self,
        selected: &[&'a WorkspaceSymbolSourceAnalysis],
        rows: &'a [WorkspaceSymbolRow],
        diagnostic: &super::super::WorkspaceSymbolDiagnostic,
    ) -> Option<Vec<&'a DocumentPathAdmission>> {
        let mut admissions = selected
            .iter()
            .map(|analysis| WorkspaceSymbolSourceAnalysis::admission(analysis))
            .collect::<Vec<_>>();
        // The analysis seed cannot renew an independently captured source-read
        // mapping. Retain both actual receipts through the same final lease;
        // equal URI/path/text never substitutes for the original allocation.
        for (_, uri, snapshot, _) in rows {
            if let Some(admission) = snapshot.path_admission.as_deref() {
                if admission.uri() != uri {
                    diagnostic.record("lease.publish_guard.4", &[]);
                    return None;
                }
                admissions.push(admission);
            } else if self.store.workspace_source_inventory_provider()
                == super::super::vfs::WorkspaceSourceInventoryProvider::HostReported
                && uri.scheme().as_str() == "file"
            {
                // Preserve the original host-file source-receipt refusal.
                // Native and pathless/in-memory purposes remain independent.
                diagnostic.record("lease.publish_guard.5", &[]);
                return None;
            }
        }
        Some(admissions)
    }
}

fn workspace_symbol_index_snapshot_is_current(
    uri: &Uri,
    snapshot: &super::super::DocumentState,
    indexed_revision: Option<u64>,
    live: Option<&super::super::DocumentState>,
    index: &tcl_lsp_core::workspace_index::WorkspaceIndex,
) -> bool {
    let Some(advanced) = workspace_symbol_snapshot_advanced(snapshot, live) else {
        return false;
    };
    // An unchanged or closed source keeps its captured index revision.
    advanced
        || match indexed_revision {
            Some(revision) => index.document_revision(uri.as_str()) == Some(revision),
            None => !index.contains_document(uri.as_str()),
        }
}

fn workspace_symbol_snapshot_advanced(
    snapshot: &super::super::DocumentState,
    live: Option<&super::super::DocumentState>,
) -> Option<bool> {
    Some(match live {
        Some(live) if live.backing_file_deleted => return None,
        // The buffer moved on after capture; its index entry
        // may follow it. The captured revision still answers.
        Some(live) if open_document_advanced(snapshot, live) => true,
        Some(live) => {
            if live.revision != snapshot.revision
                || live.version != snapshot.version
                || live.raw() != snapshot.raw()
                || live.dialect != snapshot.dialect
                || live.path_admission != snapshot.path_admission
            {
                return None;
            }
            false
        }
        None if snapshot.version.is_some() => return None,
        None => false,
    })
}
