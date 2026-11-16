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

//! Publication of an editor buffer: its source, a dialect change and its close.

use super::super::analysis::FreshAnalysisSeed;
use super::super::documents::{DocumentPathAdmission, DocumentPublication, DocumentSourceResult};
use super::super::{Backend, DocumentState, vfs};
use super::dependency_waits::{LivePublicationRetry, LivePublicationWait};
use super::{
    ClosePublicationLocks, LiveSourceContents, LiveSourceLocks, OpenPublicationLocks,
    PublishedSourcePaths, ReopenPublicationLocks,
};
use std::future::Future;
use std::sync::Arc;
use tower_lsp_server::ls_types::Uri;

impl Backend {
    /// Publish one already-visible editor buffer to Salsa and then seed its
    /// live workspace-index view if it is still the revision this `didOpen`
    /// installed.
    ///
    /// Suspending on the Salsa mutex while owning both the document-sync turn
    /// and the open-document map wedges every request behind
    /// `edits_settled()`. `did_open` therefore installs the authoritative
    /// buffer and drops that global turn before entering this function, and
    /// this deferred transaction owns neither the turn nor any partial lock
    /// while it retries.
    ///
    /// The Salsa setter runs with neither the edit turn nor the document map
    /// held, and only after the tracked snapshot census is empty. Because the
    /// acquired bundle already contains `db`, that zero remains stable through
    /// the setter and it cannot block in `cancel_others` while owning db/files.
    /// A per-publication gate preserves source edit order; currency checks
    /// before and after the index seed make a newer edit/close authoritative.
    /// The stale disk view was retired before the edit turn advanced;
    /// replacing it with live analysis before marking publication `Indexed`
    /// means a cross-document request drained behind `didOpen` cannot lose
    /// the opened file's facts. Semantic tokens proceed earlier at the
    /// `Salsa` state and retain their bounded coarse fallback; other documents
    /// remain serviceable throughout.
    pub(in super::super) async fn commit_open_document(
        &self,
        uri: &Uri,
        text: &str,
        dialect: &str,
        version: i32,
    ) -> DocumentSourceResult<bool> {
        Box::pin(self.commit_open_document_with_seed(
            uri,
            text,
            dialect,
            version,
            self.fresh_analysis_seed(uri, Arc::from(text), dialect.to_owned()),
        ))
        .await
    }

    /// Commit the source tier, release its serialisation gate, then await the
    /// supplied independent index analysis and publish it if still current.
    ///
    /// Taking the seed as a future makes the supersession boundary directly
    /// testable: neither the publication gate nor a Salsa snapshot may live
    /// while this future is pending.
    #[allow(clippy::too_many_lines)] // one request handler, step by step
    pub(in super::super) async fn commit_open_document_with_seed<F>(
        &self,
        uri: &Uri,
        text: &str,
        dialect: &str,
        version: i32,
        seed: F,
    ) -> DocumentSourceResult<bool>
    where
        F: Future<Output = DocumentSourceResult<FreshAnalysisSeed>>,
    {
        let admission = self.document_path_admission(uri).await?;
        {
            let mut publication = self.live_publication_gate.lock().await;
            let generation = self
                .live_publication_generation
                .load(std::sync::atomic::Ordering::Acquire);
            let mut snapshot_drain = None;
            let mut retry =
                LivePublicationRetry::new(generation, self.live_publication_uri_generation(uri));
            loop {
                if let Some(docs) = self.documents.try_lock("did_open_currency") {
                    if !docs
                        .get(uri)
                        .is_some_and(|doc| doc.matches_open_publication(text, dialect, version))
                    {
                        return Ok(false);
                    }
                    self.refresh_live_publication_generation(uri, &mut retry);
                }
                let locks = match self.try_reopen_publication_locks() {
                    Ok(locks) => locks,
                    Err(wait) => {
                        self.update_snapshot_drain(wait, &mut snapshot_drain);
                        publication = match self
                            .pause_open_publication(publication, "did_open", uri, wait, &mut retry)
                            .await
                        {
                            Some(publication) => publication,
                            None => return Ok(false),
                        };
                        continue;
                    }
                };

                let Some(docs) = self.documents.try_lock("did_open") else {
                    drop(locks);
                    publication = match self
                        .pause_open_publication(
                            publication,
                            "did_open",
                            uri,
                            LivePublicationWait::Documents,
                            &mut retry,
                        )
                        .await
                    {
                        Some(publication) => publication,
                        None => return Ok(false),
                    };
                    continue;
                };

                let current = docs
                    .get(uri)
                    .is_some_and(|doc| doc.matches_open_publication(text, dialect, version));
                if !current {
                    return Ok(false);
                }
                drop(docs);

                {
                    let ReopenPublicationLocks {
                        source,
                        mut last_semantic_tokens,
                        mut semantic_tokens_refresh_asked,
                        mut workspace_class_analyses,
                    } = locks;
                    Self::set_live_db_source_locked(
                        source,
                        LiveSourceContents {
                            admission: &admission,
                            text,
                            dialect,
                            project_member: true,
                        },
                        self.store.as_ref(),
                        &mut || {
                            last_semantic_tokens.remove(uri);
                            semantic_tokens_refresh_asked.remove(uri);
                            workspace_class_analyses.remove(uri);
                        },
                        self.source_path_admissions.as_ref(),
                    )?;
                }
                drop(snapshot_drain.take());

                // Semantic tokens need the current Salsa source, but deliberately
                // do not need the workspace-index seed below: their enriched read
                // is budgeted and their fallback tokenises these same live bytes.
                // Publish that narrower readiness boundary now, so a cold large
                // file can paint promptly while its cross-document facts finish.
                if !self
                    .mark_open_salsa_published_if_current(uri, text, dialect, version)
                    .await?
                {
                    return Ok(false);
                }
                break;
            }
        }

        // The immediate post-turn retirement above deliberately leaves a
        // short *missing* index slot rather than a stale one while Salsa is
        // unavailable. Once the source is published, fill that slot from the
        // authoritative live analysis before announcing completion. This
        // future owns no Salsa snapshot and runs after the publication gate is
        // released, so a newer edit can publish and make this seed obsolete.
        // The diagnostics worker may later replace it with its refined view.
        // A newer edit makes this seed obsolete before it can publish; stop
        // waiting on (and retrying) its whole-document analysis then.
        let seed = std::pin::pin!(seed);
        let Some(seed) = self
            .until_document_superseded(uri, seed, |doc| {
                doc.matches_open_publication(text, dialect, version)
            })
            .await
        else {
            return Ok(false);
        };
        let seed = seed?;
        self.publish_open_index_if_current(uri, text, dialect, version, seed)
            .await
    }

    /// Publish the Salsa source for one already-visible live revision.
    ///
    /// Used by `didChange` after it has committed the buffer splice and handed
    /// on the edit-order turn. A later edit or close changes the currency check
    /// and makes this older publication a no-op; the latest handler publishes
    /// the authoritative source. No lock or snapshot wait occurs while the
    /// global turn or document map is held.
    pub(in super::super) async fn commit_live_source(
        &self,
        operation: &'static str,
        uri: &Uri,
        text: &str,
        dialect: &str,
        revision: u64,
        index_needs_seed: bool,
    ) -> DocumentSourceResult<bool> {
        let admission = self.document_path_admission(uri).await?;
        let mut publication = self.live_publication_gate.lock().await;
        let generation = self
            .live_publication_generation
            .load(std::sync::atomic::Ordering::Acquire);
        let mut snapshot_drain = None;
        let mut retry =
            LivePublicationRetry::new(generation, self.live_publication_uri_generation(uri));
        loop {
            if self.live_publication_uri_generation(uri) != retry.uri_generation {
                return Ok(false);
            }
            if let Some(docs) = self.documents.try_lock(operation) {
                if !docs
                    .get(uri)
                    .is_some_and(|doc| doc.matches_live_source_publication(text, dialect, revision))
                {
                    return Ok(false);
                }
                self.refresh_live_publication_generation(uri, &mut retry);
            }
            let locks = match self.try_live_source_locks() {
                Ok(locks) => locks,
                Err(wait) => {
                    self.update_snapshot_drain(wait, &mut snapshot_drain);
                    publication = match self
                        .pause_open_publication(publication, operation, uri, wait, &mut retry)
                        .await
                    {
                        Some(publication) => publication,
                        None => return Ok(false),
                    };
                    continue;
                }
            };
            let Some(docs) = self.documents.try_lock(operation) else {
                drop(locks);
                publication = match self
                    .pause_open_publication(
                        publication,
                        operation,
                        uri,
                        LivePublicationWait::Documents,
                        &mut retry,
                    )
                    .await
                {
                    Some(publication) => publication,
                    None => return Ok(false),
                };
                continue;
            };
            let Some(current) = docs.get(uri).filter(|doc| {
                doc.matches_live_source_publication(text, dialect, revision)
                    && self.live_publication_uri_generation(uri) == retry.uri_generation
            }) else {
                return Ok(false);
            };
            let orphaned = current.backing_file_deleted;
            drop(docs);

            Self::set_live_db_source_locked(
                locks,
                LiveSourceContents {
                    admission: &admission,
                    text,
                    dialect,
                    project_member: !orphaned,
                },
                self.store.as_ref(),
                &mut || {},
                self.source_path_admissions.as_ref(),
            )?;
            drop(snapshot_drain.take());
            let mut docs = self.documents.lock(operation).await;
            let Some(doc) = docs.get_mut(uri) else {
                return Ok(false);
            };
            if !doc.matches_live_source_publication(text, dialect, revision)
                || self.live_publication_uri_generation(uri) != retry.uri_generation
            {
                return Ok(false);
            }
            DocumentPathAdmission::with_current(
                &[admission.as_ref()],
                self.store.as_ref(),
                || {
                    doc.publication = if index_needs_seed && !doc.backing_file_deleted {
                        DocumentPublication::Salsa
                    } else {
                        DocumentPublication::Indexed
                    };
                },
            )?;
            drop(docs);
            self.live_publication_advanced.notify_waiters();
            return Ok(true);
        }
    }

    fn retire_dialect_source(
        &self,
        locks: OpenPublicationLocks<'_>,
        uri: &Uri,
        admission: &DocumentPathAdmission,
        text: &str,
        dialect: &str,
        orphaned: bool,
    ) -> DocumentSourceResult<()> {
        let OpenPublicationLocks { source, mut index } = locks;
        Self::set_live_db_source_locked(
            source,
            LiveSourceContents {
                admission,
                text,
                dialect,
                project_member: !orphaned,
            },
            self.store.as_ref(),
            &mut || {
                index.remove_document(uri.as_str());
            },
            self.source_path_admissions.as_ref(),
        )
    }

    fn mark_live_dialect_publication(
        &self,
        admission: &DocumentPathAdmission,
        doc: &mut DocumentState,
    ) -> DocumentSourceResult<()> {
        DocumentPathAdmission::with_current(&[admission], self.store.as_ref(), || {
            doc.publication = if doc.backing_file_deleted {
                DocumentPublication::Indexed
            } else {
                DocumentPublication::Salsa
            };
        })
    }

    /// Publish a dialect change for one already-visible live revision and
    /// retire the workspace-index facts built under its previous dialect.
    ///
    /// This is the indexed counterpart of [`Self::commit_live_source`]. It is
    /// used after the document map has recorded a dialect change, never while
    /// an edit-order turn or document-map guard is held. The revision/text/
    /// dialect check makes a newer edit, close, or re-open authoritative. The
    /// old index slot is removed at the Salsa boundary, then independently
    /// rebuilt before ordinary workspace-aware readers are released.
    pub(in super::super) async fn commit_live_dialect(
        &self,
        operation: &'static str,
        uri: &Uri,
        text: &str,
        dialect: &str,
        revision: u64,
    ) -> DocumentSourceResult<bool> {
        let admission = self.document_path_admission(uri).await?;
        let mut publication = self.live_publication_gate.lock().await;
        let generation = self
            .live_publication_generation
            .load(std::sync::atomic::Ordering::Acquire);
        let mut snapshot_drain = None;
        let mut retry =
            LivePublicationRetry::new(generation, self.live_publication_uri_generation(uri));
        loop {
            if self.live_publication_uri_generation(uri) != retry.uri_generation {
                return Ok(false);
            }
            if let Some(docs) = self.documents.try_lock(operation) {
                if !docs
                    .get(uri)
                    .is_some_and(|doc| doc.matches_live_source_publication(text, dialect, revision))
                {
                    return Ok(false);
                }
                self.refresh_live_publication_generation(uri, &mut retry);
            }
            let locks = match self.try_open_publication_locks() {
                Ok(locks) => locks,
                Err(wait) => {
                    self.update_snapshot_drain(wait, &mut snapshot_drain);
                    publication = match self
                        .pause_open_publication(publication, operation, uri, wait, &mut retry)
                        .await
                    {
                        Some(publication) => publication,
                        None => return Ok(false),
                    };
                    continue;
                }
            };
            let Some(docs) = self.documents.try_lock(operation) else {
                drop(locks);
                publication = match self
                    .pause_open_publication(
                        publication,
                        operation,
                        uri,
                        LivePublicationWait::Documents,
                        &mut retry,
                    )
                    .await
                {
                    Some(publication) => publication,
                    None => return Ok(false),
                };
                continue;
            };
            let Some(current) = docs
                .get(uri)
                .filter(|doc| doc.matches_live_source_publication(text, dialect, revision))
            else {
                return Ok(false);
            };
            let orphaned = current.backing_file_deleted;
            drop(docs);

            self.retire_dialect_source(locks, uri, &admission, text, dialect, orphaned)?;
            drop(snapshot_drain.take());
            let mut docs = self.documents.lock(operation).await;
            let Some(doc) = docs.get_mut(uri) else {
                return Ok(false);
            };
            if !doc.matches_live_source_publication(text, dialect, revision)
                || self.live_publication_uri_generation(uri) != retry.uri_generation
            {
                return Ok(false);
            }
            self.mark_live_dialect_publication(&admission, doc)?;
            let orphaned = doc.backing_file_deleted;
            drop(docs);
            self.live_publication_advanced.notify_waiters();
            if orphaned {
                drop(snapshot_drain);
                drop(publication);
                return Ok(true);
            }
            // Fresh analysis must not retain the live-publication gate: a
            // newer edit is authoritative and must be able to supersede this
            // seed while it runs, exactly as on the cold-open path.
            drop(snapshot_drain);
            drop(publication);
            let text: Arc<str> = Arc::from(text);
            let seed = self
                .fresh_analysis_seed(uri, Arc::clone(&text), dialect.to_owned())
                .await?;
            return self
                .publish_live_index_if_current(uri, &text, dialect, revision, seed)
                .await;
        }
    }

    /// Retire a closed URI's live derived state without waiting while holding
    /// either the edit-order turn or a partial lock bundle. A racing re-open
    /// wins the final currency check and keeps all of its newly-armed caches.
    pub(in super::super) async fn commit_closed_live_state(
        &self,
        uri: &Uri,
        admission: &DocumentPathAdmission,
    ) -> DocumentSourceResult<bool> {
        admission.check_uri(uri)?;
        let mut publication = self.live_publication_gate.lock().await;
        let generation = self
            .live_publication_generation
            .load(std::sync::atomic::Ordering::Acquire);
        let mut retry =
            LivePublicationRetry::new(generation, self.live_publication_uri_generation(uri));
        loop {
            if let Some(docs) = self.documents.try_lock("did_close_currency") {
                if docs.contains_key(uri) {
                    return Ok(false);
                }
                self.refresh_live_publication_generation(uri, &mut retry);
            }
            let locks = match self.try_close_publication_locks() {
                Ok(locks) => locks,
                Err(wait) => {
                    publication = match self
                        .pause_open_publication(publication, "did_close", uri, wait, &mut retry)
                        .await
                    {
                        Some(publication) => publication,
                        None => return Ok(false),
                    };
                    continue;
                }
            };
            let Some(docs) = self.documents.try_lock("did_close") else {
                drop(locks);
                publication = match self
                    .pause_open_publication(
                        publication,
                        "did_close",
                        uri,
                        LivePublicationWait::Documents,
                        &mut retry,
                    )
                    .await
                {
                    Some(publication) => publication,
                    None => return Ok(false),
                };
                continue;
            };
            if docs.contains_key(uri) {
                return Ok(false);
            }
            let ClosePublicationLocks {
                mut index,
                mut diag_slots,
                mut last_semantic_tokens,
                mut semantic_tokens_refresh_asked,
                mut workspace_class_analyses,
            } = locks;
            DocumentPathAdmission::with_current(&[admission], self.store.as_ref(), || {
                index.remove_document(uri.as_str());
                if let Some(slot) = diag_slots.get_mut(uri) {
                    if slot.running {
                        slot.latest_inputs = None;
                        slot.dirty = false;
                    } else {
                        diag_slots.remove(uri);
                    }
                }
                last_semantic_tokens.remove(uri);
                semantic_tokens_refresh_asked.remove(uri);
                workspace_class_analyses.remove(uri);
            })?;
            drop(docs);
            return Ok(true);
        }
    }

    /// Publish one live source while keeping local Salsa handles distinct from
    /// cross-document project membership for edited open orphans.
    pub(in super::super) fn set_live_db_source_locked(
        locks: LiveSourceLocks<'_>,
        source: LiveSourceContents<'_>,
        store: &dyn vfs::SourceStore,
        publish_derived: &mut dyn FnMut(),
        published_paths: &PublishedSourcePaths,
    ) -> DocumentSourceResult<()> {
        let LiveSourceContents {
            admission,
            text,
            dialect,
            project_member,
        } = source;
        admission.check_current(store)?;
        let uri = admission.uri();
        let LiveSourceLocks {
            mut db,
            mut files,
            mut tombstones,
            mut project_members,
            mut project,
            global_config,
            folder_configs,
        } = locks;
        DocumentPathAdmission::with_current(&[admission], store, || {
            let source_membership_changed = Self::set_admitted_db_sources_locked(
                &mut db,
                &mut files,
                &mut tombstones,
                std::iter::once((admission, text, dialect)),
                published_paths,
            );
            let (removals, additions) = if project_member {
                (None, Some(uri))
            } else {
                (Some(uri), None)
            };
            Self::sync_db_project_membership_with_configurations(
                &mut db,
                &files,
                &mut project_members,
                &mut project,
                (*global_config, &folder_configs),
                (removals, additions, source_membership_changed),
            );
            publish_derived();
        })?;
        Ok(())
    }
}
