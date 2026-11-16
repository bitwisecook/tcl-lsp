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

//! The workspace-level request handlers: folders, watched files, file renames and symbols.

use crate::{
    Backend, DidChangeWatchedFilesParams, DidChangeWorkspaceFoldersParams, HashMap, HashSet,
    MessageType, ReloadTrigger, RenameFilesParams, Uri, WatchedFileChanges, WorkspaceEdit,
    WorkspaceSymbolDiagnostic, WorkspaceSymbolParams, WorkspaceSymbolResponse, core_file_ops,
    jsonrpc, partition_watched_file_changes, readonly_reference_reports, uri_norm,
};
use std::str::FromStr;

impl Backend {
    pub(crate) async fn serve_did_change_workspace_folders(
        &self,
        params: DidChangeWorkspaceFoldersParams,
    ) {
        // Update the tracked folder set, then reconcile the cross-document
        // index and configuration so multi-root behaviour is not frozen at the
        // `initialize` snapshot.  Drops removed-folder state, loads/scans
        // added folders, and re-pulls config.
        let removed: Vec<Uri> = params.event.removed.iter().map(|f| f.uri.clone()).collect();
        {
            let _inputs = self.analyser_inputs_gate.write().await;
            let mut folders = self.workspace_folders.lock().await;
            for uri in &removed {
                folders.retain(|f| f != uri);
            }
            for added in &params.event.added {
                if !folders.iter().any(|f| f == &added.uri) {
                    folders.push(added.uri.clone());
                }
            }
        }
        // Drop index entries for files under removed folders so their symbols
        // stop resolving across the workspace.  Open documents are left alone —
        // their live buffer is the source of truth regardless of folder set.
        if !removed.is_empty() {
            match self.drop_index_under_folders(&removed).await {
                Ok(value) => value,
                Err(error) => {
                    self.client
                        .log_message(MessageType::WARNING, error.to_string())
                        .await;
                    return;
                }
            }
        }
        // Pick up files under any newly-added folder.  `scan_workspace_folders`
        // walks the *current* folder list and skips open documents, so it is
        // safe to call here.
        if !params.event.added.is_empty() {
            match self.scan_workspace_folders().await {
                Ok(value) => value,
                Err(error) => {
                    self.client
                        .log_message(MessageType::WARNING, error.to_string())
                        .await;
                    return;
                }
            }
        }
        // A folder add/remove shifts the salsa `Project`, `workspace_index` and
        // `package_resolver` (the cross-file resolution domain *and* the
        // always-on W120/W123 workspace-refinement inputs) without an open
        // document's own edit, so reschedule every open document — otherwise a
        // push-diagnostic client keeps a stale suppressed-W123 / cross-file
        // arity, or a false W120 the refinement would now resolve, from a
        // now-removed (or newly-added) folder. Unconditional
        // `reschedule_all_open_documents` (not a narrower `crossFileResolution`-only
        // helper): the W120/W123 refinement runs for every document regardless
        // of that opt-in toggle.
        if !removed.is_empty() || !params.event.added.is_empty() {
            self.reschedule_all_open_documents().await;
        }
        // Per-folder config may differ between roots; re-pull so the resolved
        // settings reflect the new folder set.
        match self.pull_and_apply_config().await {
            Ok(value) => value,
            Err(error) => {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
            }
        }
    }

    pub(crate) async fn serve_did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        let Some(event_admissions) = self.capture_watched_event_admissions(&params.changes).await
        else {
            return;
        };
        // tower-lsp-server dispatches notification handlers concurrently.
        // Preserve watched-event transaction order so an older deletion cannot
        // finish its index/Salsa removal after a newer create has revived the
        // same still-open path.
        let _watched_change = self.watched_file_change_gate.lock().await;
        // External (non-editor) file changes — `git checkout`, a generated
        // file, a deletion — must refresh the cross-document index so
        // definition / references / rename / call-hierarchy keep seeing the
        // project's true on-disk state between restarts.
        let WatchedFileChanges {
            config_changed,
            sidecar_changed,
            spec_pack_changed,
            last_kind,
        } = partition_watched_file_changes(params.changes);
        if sidecar_changed {
            self.invalidate_sidecar_stubs().await;
        }
        // A pack file moved: reload the whole set (a pack is a logical unit
        // spread over any number of files, so there is no such thing as
        // reloading one file of it) and, when the content really changed,
        // re-analyse every open document — a command that just became known,
        // or just stopped being known, changes what W123 and hover say about
        // code nobody touched.
        if spec_pack_changed
            && (match self.reload_spec_packs(ReloadTrigger::WatchedFile).await {
                Ok(value) => value,
                Err(error) => {
                    self.client
                        .log_message(MessageType::WARNING, error.to_string())
                        .await;
                    return;
                }
            })
        {
            self.reschedule_all_open_documents().await;
        }
        // Partition the (deduplicated) event set so the disk-backed work below
        // runs once per kind — a parallel read+analyse and one batched
        // index/db mutation — instead of once per file: a 500-file branch
        // switch would otherwise run 500 sequential full analyses here, each
        // with its own `documents.lock()` / pull-cache probe.
        let mut deleted: Vec<Uri> = Vec::new();
        // The subset of `deleted` whose buffer is still open. Their index entry
        // is retired like any other deletion, but they must not have their
        // *diagnostics* cleared as a closed file's badge would be — the editor
        // is still showing the buffer.
        let mut deleted_while_open: HashSet<Uri> = HashSet::new();
        let mut changed: Vec<Uri> = Vec::new();
        // Open buffers whose backing file reappeared (a branch switch that
        // deletes then restores a path emits CREATED/CHANGED for it): clear the
        // orphan mark so the buffer rejoins the cross-document index.
        let mut revived_while_open: Vec<Uri> = Vec::new();
        self.partition_open_watched_paths(
            last_kind,
            &mut deleted,
            &mut deleted_while_open,
            &mut changed,
            &mut revived_while_open,
        )
        .await;
        // Apply the orphan marks before any re-analysis is scheduled, so the
        // publish path (which consults the flag under the `documents` lock)
        // cannot re-add a just-retired path.
        let (revived_publications, deleted_revisions) = match self
            .update_watched_open_paths(&deleted_while_open, &revived_while_open, &event_admissions)
            .await
        {
            Ok(result) => result,
            Err(error) => {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        };
        let domain_changed =
            !deleted.is_empty() || !changed.is_empty() || !revived_publications.is_empty();

        // Closed files that already carry a badge (a pull-cache entry) and whose
        // on-disk change must refresh or clear that badge — computed once
        // under one lock rather than once per file, and applied after the whole
        // batch has updated the resolution domain, so each refresh analyses
        // against the final on-disk state — never a half-applied one when
        // several files change together.
        let (closed_badge_clear, closed_badge_refresh): (Vec<Uri>, Vec<Uri>) = {
            let cache = self.pull_diag_cache.lock().await;
            Self::watched_closed_badge_uris(&cache, &deleted, &changed, &deleted_while_open)
        };

        // DELETED: atomically retire the batch from the index and salsa
        // `Project` so a deleted file stops suppressing W123 / driving the
        // arity error cross-file. Unlike scan/folder publication, deletion is
        // authoritative even for an open path: the buffer remains usable but
        // its dead filesystem identity must leave cross-file resolution.
        if !self
            .retire_watched_deleted_paths(&deleted, &deleted_revisions, &event_admissions)
            .await
        {
            return;
        }
        // CREATED / CHANGED: read + analyse every file across the bounded
        // worker pool, then one batched index/db merge.
        match self
            .batch_reindex_from_disk_using(&changed, Some(&event_admissions))
            .await
        {
            Ok(value) => value,
            Err(error) => {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        }
        // An open buffer whose path was previously orphaned remains the source
        // of truth when that path reappears. Restore both its live Salsa input
        // and standalone index view; a racing edit/close/delete wins through
        // the ordinary revision and orphan currency checks.
        if !self
            .restore_revived_live_documents(revived_publications)
            .await
        {
            return;
        }

        // Refresh/clear the badges of the closed files that changed on disk, now
        // the resolution domain has settled.
        for uri in &closed_badge_clear {
            self.clear_closed_diagnostics(uri).await;
        }
        for uri in &closed_badge_refresh {
            self.publish_closed_file_diagnostics(uri).await;
        }
        // A watched (non-open) file's create/change/delete shifts the cross-file
        // resolution domain *and* `workspace_index` (the always-on W120/W123
        // workspace-refinement input), but no open document's own edit triggered
        // it — so a push-diagnostic client would keep stale cross-file results (a
        // suppressed W123, an arity error sourced from the now-changed file, or a
        // false W120 the refinement would now resolve via a `source` ancestor)
        // until the caller is next edited. Reschedule every open document — not
        // just `crossFileResolution`-enabled ones, since the W120/W123 refinement
        // is unconditional — so both the cross-file pass and the refinement re-run
        // against the new domain.
        if domain_changed {
            self.reschedule_all_open_documents().await;
        }
        // Re-read config.ini / .tcl-lsp.ini and re-apply with full precedence,
        // then rebuild the package database (libraryPaths may have changed) and
        // re-run open documents so the new settings take effect immediately.
        if config_changed {
            self.refresh_watched_configuration().await;
        }
    }

    pub(crate) async fn serve_symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> jsonrpc::Result<Option<WorkspaceSymbolResponse>> {
        let diagnostic = WorkspaceSymbolDiagnostic::new();
        // Workspace-wide query (no document URI): gate on the process-global
        // `workspaceSymbols` toggle.
        if !self
            .feature_toggles
            .lock()
            .await
            .is_enabled("workspaceSymbols")
        {
            diagnostic.record("endpoint.disabled", &[]);
            diagnostic.emit(&self.client, "none").await;
            return Ok(None);
        }
        // Answered from the workspace index, not from the open-document map.
        // VS Code re-issues this request on every keystroke in its Ctrl+T box,
        // so cloning the whole document store — every buffer and its
        // `LineIndex` — and serially re-analysing each open document per
        // keystroke would be costly *and* would miss every symbol in a file
        // the editor had not opened.  The index already holds each of them,
        // filters before materialising, and caps the answer.
        //
        // Freshness: the index is refreshed on each document's diagnostics
        // publish, which the debounce puts ~50 ms behind an edit.  A symbol
        // picker tolerates that — it is the same staleness every other
        // cross-document feature answers with.  What it does *not* tolerate is
        // an index that has not been populated at all yet, so wait out the
        // initial folder scan first.  The `didOpen`/`didChange`
        // barrier comes first for the same reason every other reader takes it:
        // a query issued after an open must observe that open.
        self.edits_settled().await;
        self.wait_for_workspace_scan().await?;
        let query = params.query.as_str();
        // A document indexed again while its rows were being mapped is
        // captured again; only the last attempt answers without it.
        let attempts = std::sync::atomic::AtomicUsize::new(0);
        let result = readonly_reference_reports::retry_readonly_attempt(None, || {
            let attempt = attempts.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let moved = if attempt + 1 < readonly_reference_reports::READONLY_CURRENCY_ATTEMPTS {
                readonly_reference_reports::MovedIndexRows::Recapture
            } else {
                readonly_reference_reports::MovedIndexRows::Omit
            };
            let diagnostic = &diagnostic;
            async move {
                if attempt != 0 {
                    let _ = crate::rt::timeout(
                        readonly_reference_reports::source_views::SOURCE_VIEW_CONTENTION_BACKOFF,
                        self.live_publication_advanced.notified(),
                    )
                    .await;
                }
                self.workspace_symbol_attempt_diagnostic(query, diagnostic, moved)
                    .await
            }
        })
        .await
        .map_err(Into::into);
        diagnostic
            .emit(
                &self.client,
                match &result {
                    Ok(Some(_)) => "some",
                    Ok(None) => "none",
                    Err(_) => "error",
                },
            )
            .await;
        result
    }

    /// Project source-reference edits for an explicit host file rename.
    /// Each literal proposal retains its actual FILE role and source owner;
    /// current document/index joins precede versioned edit publication.
    pub(crate) async fn serve_will_rename_files(
        &self,
        params: RenameFilesParams,
    ) -> jsonrpc::Result<Option<WorkspaceEdit>> {
        let Some(inputs) = self.file_rename_input_snapshot().await else {
            return Ok(None);
        };
        // Workspace-wide file operation: gate on the process-global
        // `workspaceFileOps` toggle.
        if !self
            .feature_toggles
            .lock()
            .await
            .is_enabled("workspaceFileOps")
        {
            return Ok(None);
        }
        let Some(roots) = self
            .workspace_folder_urls()
            .await
            .iter()
            .map(uri_norm::file_action_path)
            .collect::<Option<Vec<_>>>()
        else {
            return Ok(None);
        };
        let Some(renames) = Self::file_rename_paths(&params) else {
            return Ok(None);
        };
        // The roster schedules work only. Refresh each document from its
        // genuine current analysis, then derive proposals from that same owner.
        let (candidates, mut generation) = {
            let index = self.workspace_index.read().await;
            let mut candidates: Vec<_> = index.sources().map(|row| row.uri.clone()).collect();
            candidates.sort();
            candidates.dedup();
            (candidates, index.generation())
        };
        let mut current_sources = HashMap::new();
        for owner in candidates {
            if self
                .refresh_file_rename_source(&owner, &mut current_sources, &mut generation, &inputs)
                .await?
                .is_break()
            {
                return Ok(None);
            }
        }
        let raw_edits: Vec<core_file_ops::RenameEdit> = {
            let index = self.workspace_index.read().await;
            if index.generation() != generation {
                return Ok(None);
            }
            renames
                .iter()
                .flat_map(|(old, new)| {
                    core_file_ops::FileRenameSourceAction::new(old, new, &roots).map_or_default(
                        |action| {
                            core_file_ops::compute_rename_edits(&action, &index, &|owner| {
                                uri_norm::file_action_path(&owner.parse::<Uri>().ok()?)
                            })
                        },
                    )
                })
                .collect()
        };
        if raw_edits.is_empty() {
            return Ok(None);
        }
        let mut by_dep: HashMap<Uri, Vec<core_file_ops::RenameEdit>> = HashMap::new();
        for edit in raw_edits {
            let Ok(uri) = edit.uri.parse::<Uri>() else {
                return Ok(None);
            };
            by_dep.entry(uri).or_default().push(edit);
        }
        let Some(held) = self
            .hold_file_rename_edits(by_dep, &mut current_sources)
            .await?
        else {
            return Ok(None);
        };
        // Closed files have no client version; re-read them before the final
        // open-document/index join rather than publishing stale indexed spans.
        for (uri, document, _, _) in &held {
            let Some(current) = self.read_document(uri).await? else {
                return Ok(None);
            };
            if current.text != document.text
                || current.dialect != document.dialect
                || current.revision != document.revision
                || current.version != document.version
            {
                return Ok(None);
            }
        }
        Ok(self
            .file_rename_edit_if_current(held, generation, &inputs)
            .await)
    }

    /// `workspace/didRenameFiles`: after the client applies a
    /// rename on disk, refresh the workspace — retire the old URI entirely
    /// and re-index the renamed file from its new path so cross-document
    /// features (including future renames) stay current.
    pub(crate) async fn serve_did_rename_files(&self, params: RenameFilesParams) {
        // Same workspace-file-ops gate as `will_rename_files`.
        if !self
            .feature_toggles
            .lock()
            .await
            .is_enabled("workspaceFileOps")
        {
            return;
        }
        let mut prepared = Vec::with_capacity(params.files.len());
        for file in &params.files {
            let mut pair = Vec::with_capacity(2);
            for raw in [&file.old_uri, &file.new_uri] {
                let parsed = match Uri::from_str(raw) {
                    Ok(uri) => uri,
                    Err(error) => {
                        self.client
                            .log_message(
                                MessageType::WARNING,
                                format!("file rename URI is unavailable: {error}"),
                            )
                            .await;
                        return;
                    }
                };
                match self.document_path_admission(&parsed).await {
                    Ok(admission) => pair.push((parsed, admission)),
                    Err(error) => {
                        self.client
                            .log_message(MessageType::WARNING, error.to_string())
                            .await;
                        return;
                    }
                }
            }
            prepared.push(pair);
        }
        for pair in &prepared {
            for (uri, admission) in pair {
                if let Err(error) = admission.check_for(uri, self.store.as_ref()) {
                    self.client
                        .log_message(MessageType::WARNING, error.to_string())
                        .await;
                    return;
                }
            }
        }
        for pair in prepared {
            let mut pair = pair.into_iter();
            let (old, old_admission) = pair
                .next()
                .expect("the prepared pair includes the original URI");
            let (new, new_admission) = pair.next().expect("the prepared pair includes the new URI");
            if let Err(error) = self.retire_renamed_uri(&old, old_admission).await {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
            if let Err(error) = self
                .reindex_index_from_disk_using(&new, Some(new_admission))
                .await
            {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        }
    }
}
