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

//! Reconciling files with the namespaces their `source` sites run them in.

use super::{
    Arc, Backend, DocumentPublication, DocumentSourceResult, DocumentSourceUnavailable,
    DocumentState, Uri, readonly_reference_reports,
};

/// A source-rehoming read that never waits for live publication while holding
/// [`crate::BackendState::rehoming_gate`].
pub(in super::super) enum RehomingDocumentSnapshot {
    /// Published source bytes for independently leased source-view capture.
    Ready(Box<DocumentState>),
    /// An open revision is waiting to acquire the rehoming gate and publish.
    Pending,
    /// The URI has neither an open buffer nor a tracked Salsa source.
    Missing,
}

impl Backend {
    /// On-disk workspace-folder scan: analyse `.tcl` / `.tm`
    /// files in the workspace folders that the editor hasn't
    /// opened, merging their definitions / invocation sites into
    /// the cross-document workspace index.
    ///
    /// Editor-opened documents already feed the index through
    /// `publish_analyser_diagnostics`; this fills the gap for
    /// project files the user hasn't opened yet so cross-document
    /// features (references / rename / call-hierarchy /
    /// completion) see the whole project, not just open tabs.
    ///
    /// The directory walk, file reads and per-file analysis run on
    /// a `spawn_blocking` worker so the LSP event loop stays
    /// responsive.  URIs already present in `self.documents` are
    /// skipped so the on-disk copy never clobbers a live (and
    /// possibly unsaved) editor buffer.  The walk is capped at
    /// `tclLsp.workspaceScan.maxFiles` files (default
    /// [`WORKSPACE_SCAN_FILE_CAP`]) so a large tree can't
    /// stall start-up.
    /// Reconcile exact source views and require their current indexed settlement.
    pub(in super::super) async fn refresh_source_rehoming(&self) -> DocumentSourceResult<()> {
        let _guard = self.rehomed_index_guard().await?;

        Ok(())
    }

    /// Reconcile source-site views and keep the transaction gate held for the
    /// caller's subsequent workspace-index snapshot. Standalone publishers use
    /// the same gate, so none can invalidate the freshly re-homed view between
    /// reconciliation and a definition/reference/rename read.
    pub(in super::super) async fn rehomed_index_guard(
        &self,
    ) -> DocumentSourceResult<tokio::sync::MutexGuard<'_, ()>> {
        // The settled views reach the workspace index in the publication that
        // follows them. A settlement taken while another publication is in
        // flight can name views that publication then replaces; those never
        // reach the index, so waiting on that settlement cannot succeed. Each
        // attempt therefore settles again and checks the views it names.
        let mut retired = "source-view work has no current physical owner";
        for attempt in 0..readonly_reference_reports::source_views::SOURCE_VIEW_CONTENTION_ATTEMPTS
        {
            if attempt != 0 {
                let _ = crate::rt::timeout(
                    readonly_reference_reports::source_views::SOURCE_VIEW_CONTENTION_BACKOFF,
                    self.live_publication_advanced.notified(),
                )
                .await;
            }
            let Some(settlement) = self.refresh_original_source_view_graph().await? else {
                break;
            };
            let guard = self.rehoming_gate.lock().await;
            if self.original_source_view_settlement_views_are_indexed(&settlement) {
                return Ok(guard);
            }
            retired = "source-view settlement retired";
        }
        let uri = self
            .workspace_index
            .read()
            .await
            .document_uris()
            .first()
            .and_then(|uri| uri.parse().ok())
            .ok_or_else(|| DocumentSourceUnavailable::InvalidStoredUri {
                original: retired.to_owned(),
            })?;
        Err(DocumentSourceUnavailable::ConditionalAdvice { uri })
    }

    /// Capture a source for rehoming without calling [`Self::read_document`].
    /// The latter waits for indexed publication, which would deadlock here: the
    /// pending publisher needs the rehoming gate its caller already owns.
    pub(in super::super) async fn rehoming_document_snapshot(
        &self,
        uri: &Uri,
    ) -> DocumentSourceResult<RehomingDocumentSnapshot> {
        let admission = self.document_path_admission(uri).await?;
        let docs = self.documents.lock("refresh_source_rehoming").await;
        let live = match docs.get(uri) {
            Some(doc) if doc.publication != DocumentPublication::Indexed => {
                return Ok(RehomingDocumentSnapshot::Pending);
            }
            Some(doc) => Some(
                doc.clone()
                    .with_path_admission(Arc::clone(&admission))
                    .normalised_for_analysis(),
            ),
            None => None,
        };
        drop(docs);
        if let Some(doc) = live {
            return Ok(RehomingDocumentSnapshot::Ready(Box::new(doc)));
        }

        let snapshot = {
            let db = self.db.lock().await;
            let files = self.db_files.lock().await;
            files.get(uri).copied().map(|file| {
                DocumentState::new(file.text(&*db).clone(), file.dialect(&*db).clone())
                    .with_path_admission(Arc::clone(&admission))
                    .normalised_for_analysis()
            })
        };
        Ok(
            if self
                .documents
                .lock("refresh_source_rehoming_currency")
                .await
                .contains_key(uri)
            {
                RehomingDocumentSnapshot::Pending
            } else if let Some(snapshot) = snapshot {
                RehomingDocumentSnapshot::Ready(Box::new(snapshot))
            } else {
                RehomingDocumentSnapshot::Missing
            },
        )
    }
}
