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

//! The workspace-index view of an open document's current revision.

use super::super::analysis::FreshAnalysisSeed;
use super::super::documents::{DocumentPathAdmission, DocumentPublication, DocumentSourceResult};
use super::super::{Backend, DocumentState};
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use tower_lsp_server::ls_types::Uri;

/// Document-map holder labels for one index publication.
struct IndexPublicationSites {
    pub(in super::super) currency: &'static str,
    pub(in super::super) complete: &'static str,
    commit: &'static str,
}

/// How long an edit waits before seeding a not-yet-indexed document's
/// workspace view, so a burst of keystrokes runs one seed analysis.
const INDEX_SEED_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(100);

impl Backend {
    /// Mark the matching `didOpen` revision safe for Salsa-backed providers.
    pub(super) async fn mark_open_salsa_published_if_current(
        &self,
        uri: &Uri,
        text: &str,
        dialect: &str,
        version: i32,
    ) -> DocumentSourceResult<bool> {
        let mut docs = self.documents.lock("did_open_salsa_published").await;
        let Some(doc) = docs.get_mut(uri) else {
            return Ok(false);
        };
        if !doc.matches_open_publication(text, dialect, version) {
            return Ok(false);
        }
        let admission = doc.path_admission.clone().map_or_else(
            || DocumentPathAdmission::capture(uri, self.store.as_ref()).map(Arc::new),
            Ok,
        )?;
        admission.check_uri(uri)?;
        DocumentPathAdmission::with_current(&[admission.as_ref()], self.store.as_ref(), || {
            doc.publication = DocumentPublication::Salsa;
        })?;
        drop(docs);
        self.live_publication_advanced.notify_waiters();
        Ok(true)
    }

    /// Seed the live index and publish full readiness only if this remains the
    /// `didOpen` revision that produced `analysis`.
    pub(in super::super) async fn publish_open_index_if_current(
        &self,
        uri: &Uri,
        text: &str,
        dialect: &str,
        version: i32,
        seed: FreshAnalysisSeed,
    ) -> DocumentSourceResult<bool> {
        self.publish_index_if_current(
            uri,
            text,
            dialect,
            seed,
            |doc| doc.matches_open_publication(text, dialect, version),
            IndexPublicationSites {
                currency: "did_open_index_currency",
                complete: "did_open_publish_complete",
                commit: "did_open_publish_complete: commit",
            },
        )
        .await
    }

    /// Publish an edited revision's replacement index view without retaining
    /// the document map while an existing index reader drains.
    pub(super) async fn publish_live_index_if_current(
        &self,
        uri: &Uri,
        text: &str,
        dialect: &str,
        revision: u64,
        seed: FreshAnalysisSeed,
    ) -> DocumentSourceResult<bool> {
        self.publish_index_if_current(
            uri,
            text,
            dialect,
            seed,
            |doc| doc.matches_live_publication(text, dialect, revision),
            IndexPublicationSites {
                currency: "did_change_index_currency",
                complete: "did_change_publish_complete",
                commit: "did_change_publish_complete: commit",
            },
        )
        .await
    }

    /// The document map is never held while the index writer is awaited. The
    /// replacement reaches the index first; a revision superseded before the
    /// final check removes its own records.
    async fn publish_index_if_current(
        &self,
        uri: &Uri,
        text: &str,
        dialect: &str,
        mut seed: FreshAnalysisSeed,
        matches: impl Fn(&DocumentState) -> bool,
        sites: IndexPublicationSites,
    ) -> DocumentSourceResult<bool> {
        seed.path_admission.check_for(uri, self.store.as_ref())?;
        let uri_generation = self.live_publication_uri_generation(uri);
        loop {
            let analyser_inputs_guard = self.analyser_inputs_gate.read().await;
            let rehoming_guard = self.rehoming_gate.lock().await;
            let class_factory_generation = self.class_factory_generation.read().await;
            if self.diag_inputs_epoch() != seed.analyser_inputs_epoch
                || *class_factory_generation != seed.class_factory_generation
            {
                drop(class_factory_generation);
                drop(rehoming_guard);
                drop(analyser_inputs_guard);
                seed = self
                    .fresh_analysis_seed(uri, Arc::from(text), dialect.to_owned())
                    .await?;
                continue;
            }
            let current = |documents: &HashMap<Uri, DocumentState>| {
                documents.get(uri).is_some_and(&matches)
                    && self.live_publication_uri_generation(uri) == uri_generation
            };
            if !current(&*self.documents.lock(sites.currency).await) {
                return Ok(false);
            }
            let mut index = self.workspace_index.write().await;
            if self.live_publication_uri_generation(uri) != uri_generation {
                return Ok(false);
            }
            let index_revision = DocumentPathAdmission::with_current(
                &[seed.path_admission.as_ref()],
                self.store.as_ref(),
                || index.replace_document_with_revision(uri.as_str(), &seed.analysis),
            )?;
            drop(index);
            self.rehomed_source_seeds.lock().await.remove(uri.as_str());
            let mut documents = self.documents.lock(sites.complete).await;
            let committed = if current(&documents) {
                // The provider mapping is authenticated with the document commit.
                DocumentPathAdmission::with_current(
                    &[seed.path_admission.as_ref()],
                    self.store.as_ref(),
                    || {
                        documents.retag(sites.commit);
                        documents
                            .get_mut(uri)
                            .expect("the document map remains locked")
                            .publication = DocumentPublication::Indexed;
                    },
                )
                .map(|()| true)
            } else {
                Ok(false)
            };
            drop(documents);
            if !matches!(committed, Ok(true)) {
                self.workspace_index
                    .write()
                    .await
                    .remove_document_if_revision(uri.as_str(), index_revision);
                return committed;
            }
            self.live_publication_advanced.notify_waiters();
            return Ok(true);
        }
    }

    /// Restore the index slot removed by a cold open when an edit supersedes
    /// that open's seed. Ordinary edits retain their preceding index view.
    /// Drive `work` while `uri` still satisfies `current`; `None` once a newer
    /// publication (or close) leaves it behind, dropping the unfinished work.
    pub(super) async fn until_document_superseded<T>(
        &self,
        uri: &Uri,
        mut work: std::pin::Pin<&mut impl Future<Output = T>>,
        current: impl Fn(&DocumentState) -> bool,
    ) -> Option<T> {
        loop {
            let advanced = self.live_publication_advanced.notified();
            tokio::pin!(advanced);
            advanced.as_mut().enable();
            if !self
                .documents
                .lock("index_seed_currency")
                .await
                .get(uri)
                .is_some_and(&current)
            {
                return None;
            }
            tokio::select! {
                biased;
                value = work.as_mut() => return Some(value),
                () = advanced => {}
            }
        }
    }

    async fn restore_missing_live_index(
        &self,
        uri: &Uri,
        text: &Arc<str>,
        dialect: &str,
        revision: u64,
        index_needs_seed: bool,
    ) -> DocumentSourceResult<bool> {
        if !index_needs_seed {
            return Ok(true);
        }
        {
            let docs = self.documents.lock("did_change_restore_index").await;
            let Some(doc) = docs.get(uri) else {
                return Ok(false);
            };
            if !doc.matches_live_source_publication(text, dialect, revision) {
                return Ok(false);
            }
            if doc.backing_file_deleted {
                return Ok(true);
            }
        }
        // Each keystroke before the first seed lands would otherwise start
        // its own whole-document analysis, every one obsolete as soon as the
        // next edit arrives. Settle briefly, and drop a seed a newer edit
        // has superseded rather than finish (or retry) it.
        let seed = std::pin::pin!(async {
            crate::rt::sleep(INDEX_SEED_DEBOUNCE).await;
            self.fresh_analysis_seed(uri, Arc::clone(text), dialect.to_owned())
                .await
        });
        let Some(seed) = self
            .until_document_superseded(uri, seed, |doc| {
                doc.matches_live_source_publication(text, dialect, revision)
            })
            .await
        else {
            return Ok(false);
        };
        let seed = seed?;
        self.publish_live_index_if_current(uri, text, dialect, revision, seed)
            .await
    }

    /// Publish an edited source and, when it superseded a missing cold-open
    /// seed, restore that index slot before declaring the edit complete.
    pub(in super::super) async fn publish_changed_document(
        &self,
        uri: &Uri,
        text: &Arc<str>,
        dialect: &str,
        revision: u64,
        index_needs_seed: bool,
    ) -> DocumentSourceResult<bool> {
        if !self
            .commit_live_source("did_change", uri, text, dialect, revision, index_needs_seed)
            .await?
        {
            return Ok(false);
        }
        self.restore_missing_live_index(uri, text, dialect, revision, index_needs_seed)
            .await
    }
}
