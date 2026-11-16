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

//! Publishing a lifted report: the index refresh, the cache commit and the timing markers.

use std::collections::{HashMap, HashSet};
use std::ops::ControlFlow;
use std::str::FromStr;
use std::sync::Arc;

use tcl_compiler::analyser::AnalysisResult;
use tcl_lsp_core::workspace_index as core_workspace_index;
use tokio::sync::Mutex;
use tower_lsp_server::ls_types::{MessageType, Uri};

use crate::DocumentState;
use crate::current_diagnostics::DiagnosticUnavailable;
use crate::tracked_locks::TrackedRwLock;
use crate::workspace_diagnostic_sources::WorkspaceDiagnosticSources;

use super::consumers::{
    IndexedDiagnosticFacts, OrphanCommandDependencies, WorkspaceCommandQueries,
    add_entry_point_diagnostic_consumers, command_diagnostic_consumers,
    original_diagnostic_consumers, orphaned_fact_consumers, source_diagnostic_consumers,
    unindexed_open_documents,
};
use super::delivery::DeliveryCtx;
use super::publisher::DiagnosticPublishOutcome;
use super::scheduling::reschedule_peers;
use super::{
    DiagnosticPublicationSources, IndexedDiagnosticChange, IssuedDiagnosticPublication,
    PublishTiming,
};

pub(in super::super) type DiagnosticProviderResult = (
    Result<(), DiagnosticUnavailable>,
    Result<Vec<tower_lsp_server::ls_types::Diagnostic>, crate::rt::JoinError>,
    Option<WorkspaceCommandQueries>,
);

#[cfg(test)]
pub(in super::super) async fn publish_diagnostics_result(
    delivery: &DeliveryCtx<'_>,
    workspace_index: &Arc<TrackedRwLock<core_workspace_index::WorkspaceIndex>>,
    rehomed_source_seeds: &Arc<Mutex<HashMap<String, Vec<String>>>>,
    rehoming_gate: &Arc<tokio::sync::Mutex<()>>,
    analysis: &Arc<AnalysisResult>,
    result: DiagnosticProviderResult,
    timing: PublishTiming<'_>,
) -> bool {
    publish_diagnostics_result_with_sources(
        delivery,
        workspace_index,
        rehomed_source_seeds,
        rehoming_gate,
        analysis,
        DiagnosticPublicationSources {
            result,
            workspace_sources: None,
        },
        timing,
    )
    .await
}

fn update_diagnostic_index(
    index: &mut core_workspace_index::WorkspaceIndex,
    docs: &HashMap<Uri, DocumentState>,
    uri: &Uri,
    analysis: &AnalysisResult,
    orphaned: bool,
) -> (IndexedDiagnosticChange, HashSet<String>) {
    let before = IndexedDiagnosticFacts::capture(index, uri.as_str());
    if orphaned {
        index.remove_document(uri.as_str());
    } else {
        index.replace_document(uri.as_str(), analysis);
    }
    let after = IndexedDiagnosticFacts::capture(index, uri.as_str());
    let change = before.change_to(&after);
    let mut consumers = command_diagnostic_consumers(index, &change.command_names);
    consumers.extend(original_diagnostic_consumers(index, docs, &change));
    if change.source_or_package_changed {
        consumers.extend(before.stale_source_consumers(index, uri.as_str()));
        consumers.extend(source_diagnostic_consumers(index, uri.as_str()));
    }
    // …plus the open documents no index query can reach. Two disjoint
    // sets, one per role: documents the index does not hold *yet*
    // (the transient open-before-first-publish race), and orphaned
    // buffers it will never hold again but which still consume what
    // this write moved. Neither can be found by the queries above, for
    // opposite reasons.
    if change.moved_workspace_facts() {
        consumers.extend(unindexed_open_documents(index, docs));
        consumers.extend(orphaned_fact_consumers(docs, &change));
    }
    (change, consumers)
}

struct DiagnosticPublicationCommit<'a, 'b> {
    pub(in super::super) delivery: &'a DeliveryCtx<'b>,
    pub(in super::super) workspace_index:
        &'a Arc<TrackedRwLock<core_workspace_index::WorkspaceIndex>>,
    pub(in super::super) rehomed_source_seeds: &'a Arc<Mutex<HashMap<String, Vec<String>>>>,
    pub(in super::super) rehoming_gate: &'a Arc<tokio::sync::Mutex<()>>,
    pub(in super::super) analysis: &'a Arc<AnalysisResult>,
    pub(in super::super) workspace_sources: &'a Option<Arc<WorkspaceDiagnosticSources>>,
    command_dependencies: &'a Option<Arc<OrphanCommandDependencies>>,
}

async fn commit_diagnostic_publication(
    commit: &DiagnosticPublicationCommit<'_, '_>,
    diags: &mut Vec<tower_lsp_server::ls_types::Diagnostic>,
) -> ControlFlow<bool, (Vec<Uri>, IssuedDiagnosticPublication)> {
    let DiagnosticPublicationCommit {
        delivery,
        workspace_index,
        rehomed_source_seeds,
        rehoming_gate,
        analysis,
        workspace_sources,
        command_dependencies,
    } = *commit;
    // A workspace query holds this gate from source-site reconciliation
    // through its final index snapshot. Take it before `documents` (the
    // same order as reconciliation's `read_document`) so this standalone
    // publish cannot replace a re-homed view in the gap between those two
    // operations. Left unordered, that gap lets declaration-side references
    // intermittently resolve `::helper` instead of `::x::helper`.
    let rehoming_guard = rehoming_gate.lock().await;
    // Hold the `documents` lock across the currency re-check, the
    // workspace-index update, AND the pull-cache/mailbox commit so a
    // concurrent `did_change`/`did_close` (which also take `documents`)
    // cannot interleave between them — one ordered commit, via the natural
    // `documents` → `workspace_index` and `documents` → `pull_diag_cache`
    // lock order. The mailbox deposit inside the lock is what stops a
    // `did_close` from landing between currency check and publication: a
    // later close replaces the pending state or follows an in-flight state
    // on the single consumer. The actual client await is below, after this
    // guard and `rehoming_guard` have been released.
    //
    // The index writer is only ever *tried* under the map: a publish
    // parked on the index while holding `documents` stalls the edit turn
    // behind the map and every request behind the turn. On contention the
    // map is released, the writer joins the index's fair queue holding
    // nothing but the rehoming gate (the order `did_open` uses), and
    // currency is re-checked under a fresh map guard before the update is applied.
    let (mut docs, mut index) = loop {
        let docs = delivery.documents.lock("publish_diagnostics_result").await;
        if !delivery.is_current(&docs).await {
            // Superseded by a newer edit (open run), a reopen, or a newer
            // closed run (generation bumped), which has taken authority
            // for this URI — settled for this version; the authoritative
            // path publishes the newer state.
            return ControlFlow::Break(true);
        }
        docs.retag("publish_diagnostics_result: workspace_index.try_write");
        if let Ok(index) = workspace_index.try_write() {
            break (docs, index);
        }
        drop(docs);
        drop(workspace_index.write().await);
    };
    if workspace_sources
        .as_ref()
        .is_some_and(|sources| !sources.matches(&docs, &index))
    {
        drop(index);
        drop(docs);
        drop(rehoming_guard);
        delivery
            .diagnostic_pass_unavailable(DiagnosticUnavailable::Stale)
            .await;
        return ControlFlow::Break(true);
    }
    if let Some(document) = docs.get_mut(delivery.uri) {
        // Missing/unavailable producers withdraw any older bounded roster.
        document
            .orphan_command_dependencies
            .clone_from(command_dependencies);
    }
    index.retag("publish_diagnostics_result: replace_document");
    // A buffer whose backing file has been deleted (an out-of-band rename /
    // removal — no `didClose` arrives) is re-analysed and re-published like
    // any other, but must not be re-added to the cross-document index: the
    // path is dead, and re-adding it here would resurrect the ghost
    // `did_change_watched_files` has just retired.
    let orphaned = docs
        .get(delivery.uri)
        .is_some_and(|doc| doc.backing_file_deleted);
    docs.retag("publish_diagnostics_result: index update");
    let (change, mut consumers) =
        update_diagnostic_index(&mut index, &docs, delivery.uri, analysis, orphaned);
    // The index is done with; nothing below reads it, and the commits below
    // await other stores.
    drop(index);
    if change.source_or_package_changed {
        docs.retag("publish_diagnostics_result: add_entry_point_diagnostic_consumers");
        add_entry_point_diagnostic_consumers(
            delivery.diag_slots,
            delivery.uri.as_str(),
            &mut consumers,
        )
        .await;
    }
    let peers: Vec<Uri> = consumers
        .into_iter()
        .filter_map(|consumer| Uri::from_str(&consumer).ok())
        .filter(|consumer| consumer != delivery.uri && docs.contains_key(consumer))
        .collect();
    // The document is now indexed standalone: invalidate its applied
    // source-site seed record so the next cross-document query
    // re-applies the seeded views.
    docs.retag("publish_diagnostics_result: rehomed_source_seeds");
    rehomed_source_seeds
        .lock()
        .await
        .remove(delivery.uri.as_str());
    drop(rehoming_guard);
    // A pull arriving after this index update but before the peer worker
    // republishes must compute against the new facts, never return the old
    // cache entry merely because the peer's text revision did not change.
    if !peers.is_empty() {
        docs.retag("publish_diagnostics_result: pull_diag_cache");
        let mut cache = delivery.pull_diag_cache.lock().await;
        for peer in &peers {
            cache.remove(peer);
        }
    }
    // Keep the pull-diagnostic cache in lock-step with the push: a
    // `textDocument/diagnostic` request now returns this exact set with
    // a fresh `result_id`, and an editor that already holds it gets a
    // cheap `Unchanged` report.
    docs.retag("publish_diagnostics_result: cache_and_enqueue");
    let Some(publication) = delivery
        .cache_and_enqueue_with_sources(
            std::mem::take(diags),
            workspace_sources.as_ref(),
            Some((workspace_index, &docs)),
        )
        .await
    else {
        drop(docs);
        delivery
            .diagnostic_pass_unavailable(DiagnosticUnavailable::Stale)
            .await;
        return ControlFlow::Break(true);
    };
    ControlFlow::Continue((peers, publication))
}

pub(in super::super) async fn publish_diagnostics_result_with_sources(
    delivery: &DeliveryCtx<'_>,
    workspace_index: &Arc<TrackedRwLock<core_workspace_index::WorkspaceIndex>>,
    rehomed_source_seeds: &Arc<Mutex<HashMap<String, Vec<String>>>>,
    rehoming_gate: &Arc<tokio::sync::Mutex<()>>,
    analysis: &Arc<AnalysisResult>,
    produced: DiagnosticPublicationSources,
    timing: PublishTiming<'_>,
) -> bool {
    let DiagnosticPublicationSources {
        result,
        workspace_sources,
    } = produced;
    let (provider_completion, result, command_queries) = result;
    let mut diags = match result {
        Ok(diags) => diags,
        Err(err) => {
            delivery
                .diagnostic_pass_unavailable(DiagnosticUnavailable::ProviderError)
                .await;
            delivery
                .client
                .log_message(
                    MessageType::WARNING,
                    format!("diagnostics worker panicked: {err}"),
                )
                .await;
            return true;
        }
    };
    let command_dependencies = if provider_completion.is_ok() {
        delivery.diagnostic_pass.and_then(|pass| {
            command_queries
                .and_then(|queries| {
                    OrphanCommandDependencies::from_pass(delivery.uri, pass, analysis, queries)
                })
                .map(Arc::new)
        })
    } else {
        None
    };
    let Some(source_inputs) = delivery
        .publication_input_guards(workspace_sources.as_deref())
        .await
    else {
        return true;
    };
    let diag_count = diags.len();
    let (peers, publication) = match commit_diagnostic_publication(
        &DiagnosticPublicationCommit {
            delivery,
            workspace_index,
            rehomed_source_seeds,
            rehoming_gate,
            analysis,
            workspace_sources: &workspace_sources,
            command_dependencies: &command_dependencies,
        },
        &mut diags,
    )
    .await
    {
        ControlFlow::Continue(publication) => publication,
        ControlFlow::Break(settled) => return settled,
    };
    // Client backpressure is isolated in the persistent publisher. Waiting for
    // the authoritative delivery preserves the timing-marker contract, but the
    // global documents map and rehoming gate have both been released first.
    drop(source_inputs);
    match publication.receipt.await {
        Ok(DiagnosticPublishOutcome::Delivered) => {}
        Ok(DiagnosticPublishOutcome::Superseded) => {
            delivery
                .diagnostic_pass_unavailable(DiagnosticUnavailable::Stale)
                .await;
            return true;
        }
        Err(_) => {
            delivery
                .diagnostic_pass_unavailable(DiagnosticUnavailable::ProviderError)
                .await;
            // The persistent publisher task was lost. Do not claim that this
            // publish reached the client; diagnostics stop, but request-critical
            // state remains live and no unsafe second consumer is spawned.
            return true;
        }
    }
    match provider_completion {
        Ok(()) => {
            delivery
                .complete_diagnostic_pass(analysis, publication.result_id)
                .await;
        }
        Err(reason) => delivery.diagnostic_pass_unavailable(reason).await,
    }
    reschedule_peers(delivery.diag_slots, delivery.uri, peers).await;
    log_publish_timing(delivery, timing, diag_count).await;
    true
}

/// Emit the two compatibility timing markers after diagnostic publication.
async fn log_publish_timing(
    delivery: &DeliveryCtx<'_>,
    timing: PublishTiming<'_>,
    diag_count: usize,
) {
    let elapsed_ms = timing.started.elapsed().as_secs_f64() * 1000.0;
    // The analyser runs a single, full ("deep") pass per publish — there is
    // no separate fast/deep split.  Emit the
    // `[timing] deep diagnostics` marker anyway so tooling that waits for the
    // deep pass to finish (the VS Code test harness' `waitForDeepDiagnostics`,
    // which keys on this exact line + `uri=`) has a reliable signal that O1xx /
    // analysis-level diagnostics for this URI have been published.
    delivery
        .client
        .log_message(
            MessageType::LOG,
            format!(
                "[timing] deep diagnostics {elapsed_ms:.0}ms \
                 (uri={uri_str}, diags={diag_count})",
                uri_str = timing.uri_str,
            ),
        )
        .await;
    delivery
        .client
        .log_message(
            MessageType::LOG,
            format!(
                "[timing] workspace_state.update {elapsed_ms:.0}ms \
                 (uri={uri_str}, lines={line_count}, diags={diag_count})",
                uri_str = timing.uri_str,
                line_count = timing.line_count,
            ),
        )
        .await;
}
