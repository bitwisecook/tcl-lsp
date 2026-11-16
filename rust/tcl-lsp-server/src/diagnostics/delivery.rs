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

//! Currency-checked delivery of one run's report to the pull cache and the client.

use std::collections::HashMap;
use std::sync::Arc;

use tcl_lsp_core::workspace_index as core_workspace_index;
use tcl_registry::CommandRegistry;
use tokio::sync::Mutex;
use tower_lsp_server::Client;
use tower_lsp_server::ls_types::Uri;

use crate::current_diagnostics::{DiagnosticCacheReceipt, DiagnosticPass, DiagnosticUnavailable};
use crate::documents::DocumentStore;
use crate::spec_packs::PublishedPackSet;
use crate::tracked_locks::TrackedRwLock;
use crate::workspace_diagnostic_sources::WorkspaceDiagnosticSources;
use crate::{DocumentState, fresh_analysis_cache, vfs};

use super::publisher::{DiagnosticPublishOutcome, DiagnosticPublisher, PackNoticeContext};
use super::pull::next_pull_diag_result_id;
use super::{DiagCurrency, DiagInputs, DiagSlot, IssuedDiagnosticPublication, PullDiagEntry};

/// The publish-side handles + per-edit identity shared by every settled
/// `run_diagnostics_core` path (master-off, F5, the analyser tail).  Borrows for
/// one call so the early-return paths and the final publish all deliver through
/// the same currency-guarded channel.
pub(in super::super) struct DeliveryCtx<'a> {
    pub(in super::super) sidecar_revision: Arc<fresh_analysis_cache::SidecarRevision>,
    pub(in super::super) inputs_epoch: Option<u64>,
    pub(in super::super) configuration_epoch: u64,
    pub(in super::super) source_inputs_epoch: Arc<std::sync::atomic::AtomicU64>,
    pub(in super::super) source_inputs_gate: Arc<tokio::sync::RwLock<()>>,
    pub(in super::super) diagnostic_pass: Option<&'a Arc<DiagnosticPass>>,
    pub(in super::super) client: &'a Client,
    pub(in super::super) diagnostic_publisher: &'a Arc<DiagnosticPublisher>,
    pub(in super::super) documents: &'a Arc<DocumentStore>,
    /// Where the APL sibling-`implementation` read gets its bytes — see
    /// [`crate::vfs`].
    pub(in super::super) store: &'a Arc<dyn vfs::SourceStore>,
    pub(in super::super) diag_slots: &'a Arc<Mutex<HashMap<Uri, DiagSlot>>>,
    pub(in super::super) pull_diag_cache: &'a Arc<Mutex<HashMap<Uri, PullDiagEntry>>>,
    /// The closed-file generation map, consulted by the currency guard for a
    /// [`DiagCurrency::ClosedFromDisk`] run.
    pub(in super::super) closed_diag_gen: &'a Arc<Mutex<HashMap<Uri, u64>>>,
    /// The source and pack/registry publication consumed by this run.
    pub(in super::super) source: &'a str,
    pub(in super::super) pack_seq: u64,
    pub(in super::super) pack_set: Option<&'a Arc<tcl_spectcl::PackSet>>,
    pub(in super::super) registry: Option<&'a Arc<CommandRegistry>>,
    pub(in super::super) spec_packs: Option<&'a Arc<Mutex<PublishedPackSet>>>,
    pub(in super::super) uri: &'a Uri,
    pub(in super::super) currency: DiagCurrency,
    pub(in super::super) version: Option<i32>,
    pub(in super::super) client_supports_pull: bool,
}

pub(super) type DiagnosticPublicationInputGuards<'a> = (
    Option<(
        tokio::sync::RwLockReadGuard<'a, ()>,
        tokio::sync::RwLockReadGuard<'a, u64>,
    )>,
    Option<tokio::sync::RwLockReadGuard<'a, ()>>,
);

pub(super) type DiagnosticSourceContext<'a> = (
    &'a Arc<TrackedRwLock<core_workspace_index::WorkspaceIndex>>,
    &'a HashMap<Uri, DocumentState>,
);

impl DeliveryCtx<'_> {
    fn inputs_are_current(&self, sidecar_required: bool) -> bool {
        let expected_epoch = if sidecar_required {
            self.inputs_epoch
        } else {
            Some(self.configuration_epoch)
        };
        expected_epoch.is_some_and(|epoch| {
            self.source_inputs_epoch
                .load(std::sync::atomic::Ordering::Acquire)
                == epoch
        }) && (!sidecar_required || self.sidecar_revision.is_published_current())
    }

    async fn input_guard(
        &self,
        sidecar_required: bool,
    ) -> Option<tokio::sync::RwLockReadGuard<'_, ()>> {
        let guard = self.source_inputs_gate.read().await;
        self.inputs_are_current(sidecar_required).then_some(guard)
    }

    pub(super) async fn publication_input_guards(
        &self,
        workspace_sources: Option<&WorkspaceDiagnosticSources>,
    ) -> Option<DiagnosticPublicationInputGuards<'_>> {
        if workspace_sources.is_some_and(|sources| !sources.is_empty()) {
            let pass = self.diagnostic_pass?;
            let guards = pass.workspace_source_input_guards().await?;
            if !self.inputs_are_current(true) {
                self.diagnostic_pass_unavailable(DiagnosticUnavailable::Stale)
                    .await;
                return None;
            }
            Some((Some(guards), None))
        } else {
            let Some(guard) = self.input_guard(true).await else {
                self.diagnostic_pass_unavailable(DiagnosticUnavailable::Stale)
                    .await;
                return None;
            };
            Some((None, Some(guard)))
        }
    }

    /// Whether this run is still the current state for `uri`, evaluated against a
    /// held `documents` snapshot.  An open run is current while the buffer is
    /// open at the captured revision; a closed-file run is current while
    /// the buffer stays closed *and* no newer closed run has bumped the per-URI
    /// generation past the one this run captured — a reopen hands authority back
    /// to the open path, and a superseding close / watched-change refresh drops
    /// this older run so it cannot publish stale diagnostics.
    ///
    /// The `closed_diag_gen` lock is taken *inside* the held `documents` lock
    /// (the `documents` → `closed_diag_gen` order; the generation is only ever
    /// bumped without `documents` held), so the nesting is cycle-free.
    pub(in super::super) async fn is_current(&self, docs: &HashMap<Uri, DocumentState>) -> bool {
        match self.currency {
            DiagCurrency::Open(revision) => docs
                .get(self.uri)
                .is_some_and(|doc| doc.revision == revision),
            DiagCurrency::ClosedFromDisk(generation) => {
                !docs.contains_key(self.uri)
                    && self.closed_diag_gen.lock().await.get(self.uri) == Some(&generation)
            }
        }
    }

    /// The `PullDiagEntry.revision` to cache this set under.  An open run caches
    /// its editor revision so a later pull can match it; a closed-file run caches
    /// the `u64::MAX` sentinel, which no live editor revision reaches — so a
    /// reopened document's pull always recomputes rather than serving a stale
    /// closed-file set.
    fn revision_for_cache(&self) -> u64 {
        match self.currency {
            DiagCurrency::Open(revision) => revision,
            DiagCurrency::ClosedFromDisk(_) => u64::MAX,
        }
    }

    fn publish_with_pack_notices<R>(
        &self,
        published: Option<&PublishedPackSet>,
        diagnostics: Vec<tower_lsp_server::ls_types::Diagnostic>,
        publish: impl FnOnce(Vec<tower_lsp_server::ls_types::Diagnostic>) -> R,
    ) -> R {
        let (Some(published), Some(packs), Some(registry)) =
            (published, self.pack_set, self.registry)
        else {
            return publish(diagnostics);
        };
        if published.seq != self.pack_seq || !Arc::ptr_eq(&published.packs, packs) {
            return publish(diagnostics);
        }
        self.diagnostic_publisher.with_pack_notices_during(
            PackNoticeContext {
                uri: self.uri,
                source: self.source,
                pack_seq: self.pack_seq,
                packs,
                registry,
                store: self.store.as_ref(),
            },
            diagnostics,
            publish,
        )
    }

    fn publish_with_required_sources<R>(
        &self,
        published: Option<&PublishedPackSet>,
        diagnostics: Vec<tower_lsp_server::ls_types::Diagnostic>,
        required: &WorkspaceDiagnosticSources,
        publish: impl FnOnce(Vec<tower_lsp_server::ls_types::Diagnostic>) -> R,
    ) -> Option<R> {
        if let (Some(published), Some(packs), Some(registry)) =
            (published, self.pack_set, self.registry)
            && published.seq == self.pack_seq
            && Arc::ptr_eq(&published.packs, packs)
        {
            return self
                .diagnostic_publisher
                .with_pack_notices_and_sources_during(
                    PackNoticeContext {
                        uri: self.uri,
                        source: self.source,
                        pack_seq: self.pack_seq,
                        packs,
                        registry,
                        store: self.store.as_ref(),
                    },
                    diagnostics,
                    Some(required),
                    publish,
                );
        }
        required.inputs(self.store.as_ref()).ok()?.publish_base(
            self.store.as_ref(),
            &mut Some(publish),
            &mut Some(diagnostics),
        )
    }

    /// Commit `diags` iff this run is still current for `uri`.
    ///
    /// The currency check, pull-cache update and mailbox deposit happen while
    /// `documents` is held. Client I/O does not: this waits for the publisher's
    /// acknowledgement only after releasing the map. A close or newer edit
    /// therefore orders before or after this commit without a check/send race.
    /// Production publishers complete their pass from the issued result id
    /// ([`Self::delivered_current_report`]); this boolean form is the tests'.
    #[cfg(test)]
    pub(in super::super) async fn deliver_if_current(
        &self,
        diags: Vec<tower_lsp_server::ls_types::Diagnostic>,
    ) -> bool {
        self.delivered_current_report(diags).await.is_some()
    }

    #[cfg(test)]
    async fn delivered_current_report(
        &self,
        diags: Vec<tower_lsp_server::ls_types::Diagnostic>,
    ) -> Option<String> {
        self.delivered_report_with_inputs(diags, true).await
    }

    pub(in super::super) async fn delivered_non_tcl_report(
        &self,
        diags: Vec<tower_lsp_server::ls_types::Diagnostic>,
    ) -> Option<String> {
        self.delivered_report_with_inputs(diags, false).await
    }

    async fn delivered_report_with_inputs(
        &self,
        diags: Vec<tower_lsp_server::ls_types::Diagnostic>,
        sidecar_required: bool,
    ) -> Option<String> {
        let publication = {
            let _inputs = self.input_guard(sidecar_required).await?;
            let docs = self.documents.lock("deliver_if_current").await;
            if !self.is_current(&docs).await {
                return None;
            }
            self.cache_and_enqueue(diags).await
        };
        match publication.receipt.await {
            Ok(DiagnosticPublishOutcome::Delivered) => Some(publication.result_id),
            Ok(DiagnosticPublishOutcome::Superseded) | Err(_) => None,
        }
    }

    /// Deliver the progressive **fast tier** — push-only, and only to a
    /// push client — iff the document is still at this run's `revision`.
    ///
    /// Deliberately *not* [`deliver_if_current`]: the fast tier must **never**
    /// prime the pull-diagnostic cache.  The pull path
    /// (`textDocument/diagnostic`) always serves or computes the *complete* deep
    /// set; a cache primed with the incomplete fast tier would let a pull in the
    /// window return a partial report.  A pull-capable client is skipped outright
    /// — its "early" signal would be a `workspace/diagnostic/refresh`, but a
    /// re-pull recomputes the full deep set synchronously, which defeats the fast
    /// tier's whole purpose, so such a client gets the deep tier's refresh
    /// instead. The currency check and mailbox deposit are atomic relative to
    /// edits and closes; the client send happens outside `documents`.
    /// Publish the fast tier for a push client, iff this run's revision is still
    /// current. Skipped for a pull client (the pull path always serves the
    /// complete deep set, never the reduced fast tier).
    pub(in super::super) async fn deliver_fast_tier_if_current(
        &self,
        diags: Vec<tower_lsp_server::ls_types::Diagnostic>,
    ) -> Option<tokio::sync::oneshot::Receiver<DiagnosticPublishOutcome>> {
        if self.client_supports_pull {
            return None;
        }
        let _inputs = self.input_guard(true).await?;
        let docs = self.documents.lock("deliver_fast_tier_if_current").await;
        if self.is_current(&docs).await {
            let published = match self.spec_packs {
                Some(spec_packs) => Some(spec_packs.lock().await),
                None => None,
            };
            // Do not wait: the deep future is pinned in this same task and must
            // keep progressing so its authoritative set can replace this one.
            Some(
                self.publish_with_pack_notices(published.as_deref(), diags, |diags| {
                    self.diagnostic_publisher.enqueue_assembled(
                        self.uri.clone(),
                        diags,
                        self.version,
                        false,
                        false,
                    )
                }),
            )
        } else {
            None
        }
    }

    /// Update the pull cache and deposit its corresponding client notification.
    ///
    /// Callers hold `documents`, making these one ordered commit relative to
    /// edits and closes. This performs no client I/O. The returned receipt must
    /// only be awaited after the document guard is released.
    pub(in super::super) async fn cache_and_enqueue(
        &self,
        diags: Vec<tower_lsp_server::ls_types::Diagnostic>,
    ) -> IssuedDiagnosticPublication {
        self.cache_and_enqueue_with_sources(diags, None, None)
            .await
            .expect("no required source inputs cannot refuse base publication")
    }

    pub(super) async fn cache_and_enqueue_with_sources(
        &self,
        diags: Vec<tower_lsp_server::ls_types::Diagnostic>,
        workspace_sources: Option<&Arc<WorkspaceDiagnosticSources>>,
        source_context: Option<DiagnosticSourceContext<'_>>,
    ) -> Option<IssuedDiagnosticPublication> {
        let published = match self.spec_packs {
            Some(spec_packs) => Some(spec_packs.lock().await),
            None => None,
        };
        // Assemble once, before the cache entry is taken: a pulled report and
        // a pushed one are the same set, so a `.tclspec`'s pack-load notices
        // have to be in the bytes the cache serves as well as in the ones the
        // publisher sends.
        let result_id = next_pull_diag_result_id();
        self.documents
            .retag_held("cache_and_enqueue: pull_diag_cache");
        let mut pull_cache = self.pull_diag_cache.lock().await;
        let _source_index = if let Some(sources) = workspace_sources {
            let (index, documents) = source_context?;
            let index = index.try_read().ok()?;
            if !sources.matches(documents, &index) {
                return None;
            }
            Some(index)
        } else {
            None
        };
        let commit = |diags: Vec<tower_lsp_server::ls_types::Diagnostic>| {
            pull_cache.insert(
                self.uri.clone(),
                PullDiagEntry {
                    currency: self.diagnostic_pass.map(|pass| {
                        DiagnosticCacheReceipt::from_publisher(pass)
                            .with_workspace_sources(workspace_sources.cloned())
                    }),
                    result_id: result_id.clone(),
                    revision: self.revision_for_cache(),
                    diagnostics: diags.clone(),
                },
            );
            self.diagnostic_publisher.enqueue_assembled(
                self.uri.clone(),
                diags,
                self.version,
                self.client_supports_pull,
                true,
            )
        };
        let receipt = match workspace_sources {
            Some(sources) => {
                self.publish_with_required_sources(published.as_deref(), diags, sources, commit)?
            }
            None => self.publish_with_pack_notices(published.as_deref(), diags, commit),
        };
        drop(pull_cache);
        drop(published);
        Some(IssuedDiagnosticPublication { result_id, receipt })
    }
}

impl DiagInputs {
    pub(in super::super) fn delivery_for_job<'a>(
        &'a self,
        diagnostic_pass: Option<&'a Arc<DiagnosticPass>>,
        source: &'a str,
        uri: &'a Uri,
        currency: DiagCurrency,
        version: Option<i32>,
    ) -> DeliveryCtx<'a> {
        DeliveryCtx {
            sidecar_revision: Arc::clone(&self.sidecar_revision),
            inputs_epoch: self.inputs_epoch,
            configuration_epoch: self.configuration_epoch,
            source_inputs_epoch: Arc::clone(&self.source_inputs_epoch),
            source_inputs_gate: Arc::clone(&self.source_inputs_gate),
            diagnostic_pass,
            client: &self.client,
            diagnostic_publisher: &self.diagnostic_publisher,
            documents: &self.documents,
            store: &self.store,
            diag_slots: &self.diag_slots,
            pull_diag_cache: &self.pull_diag_cache,
            closed_diag_gen: &self.closed_diag_gen,
            source,
            pack_seq: self.pack_seq,
            pack_set: Some(&self.pack_set),
            registry: Some(&self.registry),
            spec_packs: Some(&self.spec_packs),
            uri,
            currency,
            version,
            client_supports_pull: self.client_supports_pull,
        }
    }
}
