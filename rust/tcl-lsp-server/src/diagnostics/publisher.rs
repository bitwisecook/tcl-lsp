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

//! The persistent publisher: a latest-wins mailbox per URI, merged with pack notices.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;

use tcl_registry::CommandRegistry;
use tokio::sync::Mutex;
use tower_lsp_server::Client;
use tower_lsp_server::ls_types::{MessageType, Uri};

use crate::current_diagnostics::DiagnosticCompletionEntry;
use crate::documents::DocumentPathAdmission;
use crate::workspace_diagnostic_sources::WorkspaceDiagnosticSources;
use crate::{vfs, workspace_diagnostic_sources};

use super::scheduling::deliver_diagnostics;

/// One diagnostics notification committed after its currency check.
///
/// The outbound LSP channel is deliberately consumed by a single persistent
/// publisher.  A slow client may park that publisher, but it cannot park the
/// global document map.
pub(in super::super) struct PendingDiagnosticPublish {
    pub(in super::super) uri: Uri,
    pub(in super::super) diagnostics: Vec<tower_lsp_server::ls_types::Diagnostic>,
    pub(in super::super) version: Option<i32>,
    pub(in super::super) client_supports_pull: bool,
    announce: bool,
    delivered: tokio::sync::oneshot::Sender<DiagnosticPublishOutcome>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum DiagnosticPublishOutcome {
    Delivered,
    Superseded,
}

#[derive(Default)]
pub(in super::super) struct DiagnosticMailboxState {
    /// Latest not-yet-started publication per URI.
    pub(in super::super) pending: HashMap<Uri, PendingDiagnosticPublish>,
    /// Cross-URI commit order. Replacing a URI moves it to the tail, so the
    /// order reflects the latest surviving commit rather than the stale one it
    /// replaced.
    pub(in super::super) order: VecDeque<Uri>,
}

/// Persistent diagnostics publisher with per-URI bounded coalescing.
///
/// The mailbox is latest-wins per URI: while the client is stalled, at most one
/// not-yet-started diagnostic vector is retained for each URI.  The one item
/// already being sent cannot be cancelled safely — `SinkExt::send` may have
/// completed `start_send` before parking in `poll_flush` — so it is allowed to
/// finish and the newest queued state follows it.  This gives close/edit
/// ordering without timeout-and-retry duplicates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) enum PackNoticeSourceOwner {
    Saved(vfs::SourcePathOrigin),
    Studio(Arc<DocumentPathAdmission>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct PackNoticeEntry {
    pub(in super::super) diagnostic: tower_lsp_server::ls_types::Diagnostic,
    pub(in super::super) source: Arc<str>,
    pub(in super::super) owner: PackNoticeSourceOwner,
}

pub(in super::super) struct PreparedPackNotice {
    pub(in super::super) uri: Uri,
    pub(in super::super) diagnostic: tower_lsp_server::ls_types::Diagnostic,
    pub(in super::super) source: Arc<str>,
    pub(in super::super) owner: PackNoticeSourceOwner,
    // Retain the original per-notice path array through the caller's map insert.
    pub(in super::super) _paths: [PathBuf; 1],
}

#[derive(Default)]
pub(in super::super) struct PackNoticePublication {
    pub(in super::super) seq: u64,
    pub(in super::super) packs: Option<Arc<tcl_spectcl::PackSet>>,
    pub(in super::super) overlays: Vec<Arc<CommandRegistry>>,
    pub(in super::super) by_uri: HashMap<Uri, Vec<PackNoticeEntry>>,
}

/// Existing source and selected provider/publication owners for one notice merge.
#[derive(Clone, Copy)]
pub(in super::super) struct PackNoticeContext<'a> {
    pub(in super::super) uri: &'a Uri,
    pub(in super::super) source: &'a str,
    pub(in super::super) pack_seq: u64,
    pub(in super::super) packs: &'a Arc<tcl_spectcl::PackSet>,
    pub(in super::super) registry: &'a Arc<CommandRegistry>,
    pub(in super::super) store: &'a dyn vfs::SourceStore,
}

pub(in super::super) struct DiagnosticPublisher {
    pub(in super::super) completion: Mutex<HashMap<Uri, DiagnosticCompletionEntry>>,
    pub(in super::super) client: Client,
    pub(in super::super) state: std::sync::Mutex<DiagnosticMailboxState>,
    /// Current pack notices with their loader source and actual publication
    /// owners, rather than wire diagnostics keyed by URI alone.
    pub(in super::super) pack_notices: std::sync::Mutex<PackNoticePublication>,
    pub(in super::super) ready: tokio::sync::Notify,
    pub(in super::super) started: std::sync::atomic::AtomicBool,
}

impl DiagnosticPublisher {
    pub(in super::super) fn new(client: Client) -> Self {
        Self {
            completion: Mutex::new(HashMap::new()),
            client,
            state: std::sync::Mutex::new(DiagnosticMailboxState::default()),
            pack_notices: std::sync::Mutex::new(PackNoticePublication::default()),
            ready: tokio::sync::Notify::new(),
            started: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Merge pack notices only when their loader image and installed registry
    /// are the exact owners used by this document report.
    pub(in super::super) fn with_pack_notices(
        &self,
        context: PackNoticeContext<'_>,
        diagnostics: Vec<tower_lsp_server::ls_types::Diagnostic>,
    ) -> Vec<tower_lsp_server::ls_types::Diagnostic> {
        self.with_pack_notices_during(context, diagnostics, |diagnostics| diagnostics)
    }

    /// Rejoin the selected provider source while the synchronous publication
    /// callback commits the assembled diagnostic set.
    pub(in super::super) fn with_pack_notices_during<R>(
        &self,
        context: PackNoticeContext<'_>,
        diagnostics: Vec<tower_lsp_server::ls_types::Diagnostic>,
        publish: impl FnOnce(Vec<tower_lsp_server::ls_types::Diagnostic>) -> R,
    ) -> R {
        self.with_pack_notices_and_sources_during(context, diagnostics, None, publish)
            .expect("empty required source set always admits base publication")
    }

    pub(super) fn with_pack_notices_and_sources_during<R>(
        &self,
        context: PackNoticeContext<'_>,
        diagnostics: Vec<tower_lsp_server::ls_types::Diagnostic>,
        required: Option<&WorkspaceDiagnosticSources>,
        publish: impl FnOnce(Vec<tower_lsp_server::ls_types::Diagnostic>) -> R,
    ) -> Option<R> {
        let PackNoticeContext {
            uri,
            source,
            pack_seq,
            packs,
            registry,
            store,
        } = context;
        let required = match required {
            Some(required) => required.inputs(store).ok()?,
            None => workspace_diagnostic_sources::WorkspaceDiagnosticSourceInputs::default(),
        };
        let mut publish = Some(publish);
        let mut diagnostics = Some(diagnostics);
        let notices = self
            .pack_notices
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if notices.seq != pack_seq
            || !notices
                .packs
                .as_ref()
                .is_some_and(|published| Arc::ptr_eq(published, packs))
            || !notices
                .overlays
                .iter()
                .any(|overlay| Arc::ptr_eq(overlay, registry))
        {
            return required.publish_base(store, &mut publish, &mut diagnostics);
        }
        let matching: Vec<_> = notices
            .by_uri
            .get(uri)
            .into_iter()
            .flatten()
            .filter(|notice| notice.source.as_ref() == source)
            .collect();
        if matching.is_empty() {
            return required.publish_base(store, &mut publish, &mut diagnostics);
        }
        let saved_sources: Vec<_> = matching
            .iter()
            .filter_map(|notice| match &notice.owner {
                PackNoticeSourceOwner::Saved(origin) => Some((origin, notice.source.as_bytes())),
                PackNoticeSourceOwner::Studio(_) => None,
            })
            .collect();
        let mut studio_sources = Vec::new();
        let mut studio_inputs_available = true;
        for notice in &matching {
            if let PackNoticeSourceOwner::Studio(admission) = &notice.owner {
                if let Ok(path) = admission.current_saved_source_path(store) {
                    studio_sources.push((path, notice.source.as_bytes()));
                } else {
                    studio_inputs_available = false;
                    break;
                }
            }
        }
        let extra: Vec<_> = matching
            .iter()
            .map(|notice| notice.diagnostic.clone())
            .collect();
        let mut result = None;
        let mut commit = || {
            let mut assembled = diagnostics.take().expect("base report is available");
            assembled.extend(extra.iter().cloned());
            result = Some(publish.take().expect("publication callback is available")(
                assembled,
            ));
        };
        studio_sources.extend(required.saved.iter().copied());
        let checked = if !studio_inputs_available {
            Err(vfs::SourceUriPathUnavailable::ProviderUnavailable)
        } else if required.mappings.is_empty() {
            store.with_current_saved_source_inputs(&studio_sources, &saved_sources, &mut commit)
        } else {
            store.with_current_source_inputs(
                &required.mappings,
                &studio_sources,
                &saved_sources,
                &mut commit,
            )
        };
        if studio_inputs_available && checked.is_ok() {
            if let Some(result) = result {
                return Some(result);
            }
            unreachable!("a successful provider callback publishes exactly once");
        }
        required.publish_base(store, &mut publish, &mut diagnostics)
    }

    /// Replace the standing pack notices with a loader-backed publication and
    /// report every URI whose assembled report needs refreshing.
    pub(in super::super) fn set_pack_notices(
        &self,
        seq: u64,
        packs: Arc<tcl_spectcl::PackSet>,
        overlays: Vec<Arc<CommandRegistry>>,
        by_uri: HashMap<Uri, Vec<PackNoticeEntry>>,
    ) -> Vec<Uri> {
        let mut notices = self
            .pack_notices
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let publication_changed = notices.seq != seq
            || notices
                .packs
                .as_ref()
                .is_none_or(|old| !Arc::ptr_eq(old, &packs))
            || notices.overlays.len() != overlays.len()
            || notices
                .overlays
                .iter()
                .zip(&overlays)
                .any(|(old, fresh)| !Arc::ptr_eq(old, fresh));
        let mut touched: Vec<Uri> = notices
            .by_uri
            .keys()
            .filter(|uri| !by_uri.contains_key(*uri) || publication_changed)
            .cloned()
            .collect();
        touched.extend(
            by_uri
                .iter()
                .filter(|(uri, fresh)| {
                    publication_changed || notices.by_uri.get(*uri) != Some(*fresh)
                })
                .map(|(uri, _)| uri.clone()),
        );
        *notices = PackNoticePublication {
            seq,
            packs: Some(packs),
            overlays,
            by_uri,
        };
        touched.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        touched.dedup();
        touched
    }

    /// Commit one publication and return an acknowledgement for its eventual
    /// delivery. A superseded pending item is explicitly settled as such, so
    /// its caller cannot emit a completion marker for a state never delivered.
    pub(in super::super) fn enqueue(
        self: &Arc<Self>,
        uri: Uri,
        diagnostics: Vec<tower_lsp_server::ls_types::Diagnostic>,
        version: Option<i32>,
        client_supports_pull: bool,
        announce: bool,
    ) -> tokio::sync::oneshot::Receiver<DiagnosticPublishOutcome> {
        self.enqueue_assembled(uri, diagnostics, version, client_supports_pull, announce)
    }

    /// [`Self::enqueue`] for a set the caller has already assembled — the pull
    /// path, which has to cache the same bytes it publishes and so merges the
    /// pack notices in itself.
    pub(in super::super) fn enqueue_assembled(
        self: &Arc<Self>,
        uri: Uri,
        diagnostics: Vec<tower_lsp_server::ls_types::Diagnostic>,
        version: Option<i32>,
        client_supports_pull: bool,
        announce: bool,
    ) -> tokio::sync::oneshot::Receiver<DiagnosticPublishOutcome> {
        let (delivered, receipt) = tokio::sync::oneshot::channel();
        let publish = PendingDiagnosticPublish {
            uri: uri.clone(),
            diagnostics,
            version,
            client_supports_pull,
            announce,
            delivered,
        };
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(old) = state.pending.remove(&uri) {
            let _ = old.delivered.send(DiagnosticPublishOutcome::Superseded);
            state.order.retain(|queued| queued != &uri);
        }
        state.pending.insert(uri.clone(), publish);
        state.order.push_back(uri);
        drop(state);

        self.ensure_started();
        self.ready.notify_one();
        receipt
    }

    pub(in super::super) fn ensure_started(self: &Arc<Self>) {
        use std::sync::atomic::Ordering;

        if self
            .started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }
        let publisher = Arc::clone(self);
        drop(crate::rt::spawn(async move { publisher.run().await }));
    }

    pub(in super::super) fn pop(&self) -> Option<PendingDiagnosticPublish> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        while let Some(uri) = state.order.pop_front() {
            if let Some(publish) = state.pending.remove(&uri) {
                return Some(publish);
            }
        }
        None
    }

    pub(in super::super) async fn run(&self) {
        loop {
            // Construct the notified future before checking the queue. A notify
            // racing the empty check is then retained as the Notify permit.
            let ready = self.ready.notified();
            let Some(publish) = self.pop() else {
                ready.await;
                continue;
            };

            if publish.announce {
                self.client
                    .log_message(
                        MessageType::LOG,
                        format!(
                            "[timing] diagnostics.publish.enqueued (uri={}, version={})",
                            publish.uri.as_str(),
                            publish
                                .version
                                .map_or_else(|| "none".to_owned(), |v| v.to_string()),
                        ),
                    )
                    .await;
            }
            deliver_diagnostics(
                &self.client,
                &publish.uri,
                publish.diagnostics,
                publish.version,
                publish.client_supports_pull,
            )
            .await;
            let _ = publish.delivered.send(DiagnosticPublishOutcome::Delivered);
        }
    }
}
