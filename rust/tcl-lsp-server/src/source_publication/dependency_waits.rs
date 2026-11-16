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

//! Fair waits on the dependency that defeated an optimistic bundle.

use super::super::Backend;
use super::super::documents::EDIT_BARRIER_STALL_WARN;
use super::super::tracked_locks::SnapshotDrainGuard;
use std::future::Future;
use tower_lsp_server::ls_types::{MessageType, Uri};

/// The exact dependency a deferred `didOpen` publication last failed to
/// acquire. Kept typed so telemetry and tests use the same names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum LivePublicationWait {
    Database,
    Files,
    Tombstones,
    ProjectMembers,
    Project,
    GlobalConfig,
    FolderConfigs,
    SalsaSnapshots,
    WorkspaceIndex,
    DiagnosticsSlots,
    SemanticTokens,
    SemanticRefreshRequests,
    WorkspaceClassAnalyses,
    RehomedSourceSeeds,
    Documents,
}

pub(in super::super) struct LivePublicationRetry {
    pub(in super::super) since: crate::rt::Instant,
    pub(in super::super) retries: u64,
    reported: bool,
    pub(in super::super) generation: u64,
    pub(in super::super) uri_generation: u64,
}

impl LivePublicationRetry {
    pub(in super::super) fn new(generation: u64, uri_generation: u64) -> Self {
        Self {
            since: crate::rt::Instant::now(),
            retries: 0,
            reported: false,
            generation,
            uri_generation,
        }
    }
}

impl LivePublicationWait {
    pub(in super::super) const fn dependency(self) -> &'static str {
        match self {
            Self::Database => "db.try_lock",
            Self::Files => "db_files.try_lock",
            Self::Tombstones => "db_tombstones.try_lock",
            Self::ProjectMembers => "db_project_members.try_lock",
            Self::Project => "db_project.try_lock",
            Self::GlobalConfig => "db_config.try_lock",
            Self::FolderConfigs => "folder_db_configs.try_lock",
            Self::SalsaSnapshots => "db.snapshot_census.zero",
            Self::WorkspaceIndex => "workspace_index.try_write",
            Self::DiagnosticsSlots => "diag_slots.try_lock",
            Self::SemanticTokens => "last_semantic_tokens.try_lock",
            Self::SemanticRefreshRequests => "semantic_tokens_refresh_asked.try_lock",
            Self::WorkspaceClassAnalyses => "workspace_class_analyses.try_lock",
            Self::RehomedSourceSeeds => "rehomed_source_seeds.try_lock",
            Self::Documents => "documents.try_lock",
        }
    }
}

impl Backend {
    pub(in super::super) fn invalidate_live_publication<'a>(
        &self,
        uris: impl IntoIterator<Item = &'a Uri>,
    ) {
        let mut uri_generations = self
            .live_publication_uri_generations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for uri in uris {
            let generation = uri_generations.entry(uri.clone()).or_default();
            *generation = generation.saturating_add(1);
            self.original_source_views
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .retire_uri(uri);
        }
        drop(uri_generations);
        self.live_publication_generation
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        self.live_publication_invalidated.notify_waiters();
    }

    pub(in super::super) fn live_publication_uri_generation(&self, uri: &Uri) -> u64 {
        self.live_publication_uri_generations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(uri)
            .copied()
            .unwrap_or_default()
    }

    /// Adopt unrelated global invalidations without acquiring `documents`.
    /// The separate per-URI generation remains readable while the document map
    /// is contended, so a current publisher can still join that map's fair
    /// queue and an obsolete publisher leaves promptly after its own URI moves.
    pub(super) fn refresh_live_publication_generation(
        &self,
        uri: &Uri,
        retry: &mut LivePublicationRetry,
    ) {
        if self.live_publication_uri_generation(uri) == retry.uri_generation {
            retry.generation = self
                .live_publication_generation
                .load(std::sync::atomic::Ordering::Acquire);
        }
    }

    /// Yield after one failed deferred-open publication attempt and emit one
    /// bounded, exact diagnostic if contention persists.
    pub(in super::super) async fn pause_open_publication<'a>(
        &'a self,
        publication: tokio::sync::MutexGuard<'a, ()>,
        operation: &'static str,
        uri: &Uri,
        wait: LivePublicationWait,
        retry: &mut LivePublicationRetry,
    ) -> Option<tokio::sync::MutexGuard<'a, ()>> {
        if self.live_publication_uri_generation(uri) != retry.uri_generation {
            return None;
        }
        retry.retries = retry.retries.saturating_add(1);
        if !retry.reported && retry.since.elapsed() >= EDIT_BARRIER_STALL_WARN {
            retry.reported = true;
            let db = self.db.contention();
            let db_files = self.db_files.contention();
            self.client
                .log_message(
                    MessageType::WARNING,
                    format!(
                        "[live-publication-stall] {operation} for {} has retried {retries} times \
                         for {:.1}s; last unavailable dependency: `{operation}: {}`. The edit-order barrier and open-document \
                         map are both free; requests for other documents remain serviceable. \
                         Salsa store contention: [{db}] [{db_files}] (#1849).",
                        uri.as_str(),
                        retry.since.elapsed().as_secs_f64(),
                        wait.dependency(),
                        retries = retry.retries,
                    ),
                )
                .await;
        }
        // `try_lock` correctly avoids retaining a partial bundle, but Tokio's
        // fair mutexes do not let a try-lock barge past queued waiters. Join
        // the exact dependency's fair queue periodically, but release the
        // live-publication gate first. Readers admitted after that fair turn
        // may defeat the next optimistic bundle again; keeping the global gate
        // while repeating that cycle would starve every later live edit. A
        // newer publisher can now proceed and its per-URI generation makes
        // this older attempt retire after it reacquires the gate.
        let publication = if retry.retries.is_multiple_of(1024) {
            self.refresh_live_publication_generation(uri, retry);
            drop(publication);
            self.join_live_publication_dependency(operation, wait, retry.generation)
                .await;
            self.live_publication_gate.lock().await
        } else {
            publication
        };
        crate::rt::sleep(std::time::Duration::from_millis(1)).await;
        (self.live_publication_uri_generation(uri) == retry.uri_generation).then_some(publication)
    }

    async fn join_live_publication_dependency(
        &self,
        operation: &'static str,
        wait: LivePublicationWait,
        generation: u64,
    ) {
        match wait {
            LivePublicationWait::Database => {
                self.join_live_publication_queue(self.db.lock(), generation)
                    .await;
            }
            LivePublicationWait::Files => {
                self.join_live_publication_queue(self.db_files.lock(), generation)
                    .await;
            }
            LivePublicationWait::Tombstones => {
                self.join_live_publication_queue(self.db_tombstones.lock(), generation)
                    .await;
            }
            LivePublicationWait::ProjectMembers => {
                self.join_live_publication_queue(self.db_project_members.lock(), generation)
                    .await;
            }
            LivePublicationWait::Project => {
                self.join_live_publication_queue(self.db_project.lock(), generation)
                    .await;
            }
            LivePublicationWait::GlobalConfig => {
                self.join_live_publication_queue(self.db_config.lock(), generation)
                    .await;
            }
            LivePublicationWait::FolderConfigs => {
                self.join_live_publication_queue(self.folder_db_configs.lock(), generation)
                    .await;
            }
            // The census is not an async lock with a queue to join. The
            // bounded sleep in the caller is its fair retry point.
            LivePublicationWait::SalsaSnapshots => {}
            LivePublicationWait::WorkspaceIndex => {
                self.join_live_publication_queue(self.workspace_index.write(), generation)
                    .await;
            }
            LivePublicationWait::RehomedSourceSeeds => {
                self.join_live_publication_queue(self.rehomed_source_seeds.lock(), generation)
                    .await;
            }
            LivePublicationWait::DiagnosticsSlots => {
                self.join_live_publication_queue(self.diag_slots.lock(), generation)
                    .await;
            }
            LivePublicationWait::SemanticTokens => {
                self.join_live_publication_queue(self.last_semantic_tokens.lock(), generation)
                    .await;
            }
            LivePublicationWait::SemanticRefreshRequests => {
                self.join_live_publication_queue(
                    self.semantic_tokens_refresh_asked.lock(),
                    generation,
                )
                .await;
            }
            LivePublicationWait::WorkspaceClassAnalyses => {
                self.join_live_publication_queue(self.workspace_class_analyses.lock(), generation)
                    .await;
            }
            LivePublicationWait::Documents => {
                self.join_live_publication_queue(self.documents.lock(operation), generation)
                    .await;
            }
        }
    }

    /// Wait fairly on the exact dependency that defeated a disk publication's
    /// optimistic bundle. Callers release every publication gate first, so a
    /// continuously busy low-priority transaction cannot block live edits or
    /// source rehoming while it waits for a quiet commit window.
    async fn wait_for_disk_publication_dependency(
        &self,
        site: &'static str,
        wait: LivePublicationWait,
    ) {
        match wait {
            LivePublicationWait::Database => drop(self.db.lock().await),
            LivePublicationWait::Files => drop(self.db_files.lock().await),
            LivePublicationWait::Tombstones => drop(self.db_tombstones.lock().await),
            LivePublicationWait::ProjectMembers => {
                drop(self.db_project_members.lock().await);
            }
            LivePublicationWait::Project => drop(self.db_project.lock().await),
            LivePublicationWait::GlobalConfig => drop(self.db_config.lock().await),
            LivePublicationWait::FolderConfigs => drop(self.folder_db_configs.lock().await),
            LivePublicationWait::SalsaSnapshots => {
                crate::rt::sleep(std::time::Duration::from_millis(1)).await;
            }
            LivePublicationWait::WorkspaceIndex => drop(self.workspace_index.write().await),
            LivePublicationWait::DiagnosticsSlots => drop(self.diag_slots.lock().await),
            LivePublicationWait::SemanticTokens => {
                drop(self.last_semantic_tokens.lock().await);
            }
            LivePublicationWait::SemanticRefreshRequests => {
                drop(self.semantic_tokens_refresh_asked.lock().await);
            }
            LivePublicationWait::WorkspaceClassAnalyses => {
                drop(self.workspace_class_analyses.lock().await);
            }
            LivePublicationWait::RehomedSourceSeeds => {
                drop(self.rehomed_source_seeds.lock().await);
            }
            LivePublicationWait::Documents => drop(self.documents.lock(site).await),
        }
    }

    pub(super) async fn restart_disk_publication<'a>(
        &'a self,
        rehoming: tokio::sync::MutexGuard<'a, ()>,
        live_publication: tokio::sync::MutexGuard<'a, ()>,
        snapshot_drain: SnapshotDrainGuard,
        site: &'static str,
        wait: LivePublicationWait,
    ) -> (
        tokio::sync::MutexGuard<'a, ()>,
        tokio::sync::MutexGuard<'a, ()>,
        SnapshotDrainGuard,
    ) {
        drop(live_publication);
        drop(rehoming);
        self.wait_for_disk_publication_dependency(site, wait).await;
        let rehoming = self.rehoming_gate.lock().await;
        let live_publication = self.live_publication_gate.lock().await;
        (rehoming, live_publication, snapshot_drain)
    }

    async fn join_live_publication_queue<T>(&self, lock: impl Future<Output = T>, generation: u64) {
        let invalidated = self.live_publication_invalidated.notified();
        tokio::pin!(invalidated);
        invalidated.as_mut().enable();
        if self
            .live_publication_generation
            .load(std::sync::atomic::Ordering::Acquire)
            != generation
        {
            return;
        }
        tokio::select! {
            guard = lock => drop(guard),
            () = invalidated => {}
        }
    }
}
