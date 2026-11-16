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

//! The on-disk scan of the workspace folders into the index and the project, and the signal that its first pass has finished.

use super::*;

/// Ceiling on how many workspace files [`Backend::scan_workspace_folders`] and
/// the [`Backend::did_change_watched_files`] batch reindex analyse concurrently
/// concurrently.  Both read-and-analyse many independent on-disk files, so
/// they share this bound and the [`run_bounded`] helper; clamped to the
/// machine's parallelism (see [`WORKSPACE_WARM_MAX_CONCURRENCY`]'s rationale)
/// so a huge tree can't oversubscribe the blocking pool.
pub(super) const WORKSPACE_ANALYSIS_MAX_CONCURRENCY: usize = 16;

/// How many analysed files [`Backend::scan_workspace_folders`] accumulates
/// before merging them into `workspace_index` / the salsa `Project`.
/// Merging in batches rather than once for the whole scan bounds how many
/// `AnalysisResult`s (and their source text) are held in memory at once on a
/// large tree, while still running the *analysis* itself across the full
/// [`WORKSPACE_ANALYSIS_MAX_CONCURRENCY`]-wide pool without waiting on a batch
/// boundary.
const SCAN_MERGE_BATCH_SIZE: usize = 200;

/// One-shot readiness signal for the first [`Backend::scan_workspace_folders`]
/// pass.
///
/// [`crate::BackendState::workspace_scan_gate`] lets a request wait out a scan that is
/// *already running*, which is enough for a handler reached long after
/// startup.  It is not enough at startup: `initialized` pulls the client
/// config and registers file watchers — two client round-trips — **before** it
/// starts the scan, and a request that lands in that window acquires a free
/// gate and answers against an empty index — the first `workspace/symbol`
/// query of a session returns nothing while the second, moments later, returns
/// the same file's symbols.  This signal is what a
/// cross-file handler waits on instead: it is only released once a scan has
/// actually completed, so "the scan has not begun yet" and "the scan is in
/// flight" both wait.
#[derive(Debug, Default)]
pub(in super::super) struct WorkspaceScanReadiness {
    pub(in super::super) complete: OneShotSignal,
    /// 0 while pending, 1 after a completed scan, 2 after startup failed.
    pub(in super::super) outcome: std::sync::atomic::AtomicU8,
    pub(in super::super) shutdown: OneShotSignal,
}

impl WorkspaceScanReadiness {
    /// Record that a scan pass has finished and wake every waiter.
    pub(in super::super) fn mark_complete(&self) {
        self.outcome.store(1, std::sync::atomic::Ordering::Release);
        self.complete.raise();
    }

    pub(in super::super) fn is_complete(&self) -> bool {
        self.outcome.load(std::sync::atomic::Ordering::Acquire) == 1
    }

    fn mark_failed(&self) {
        if self
            .outcome
            .compare_exchange(
                0,
                2,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
            )
            .is_ok()
        {
            self.complete.raise();
        }
    }

    /// Wait until the initial scan succeeds or startup fails.
    pub(in super::super) async fn wait(&self) -> bool {
        self.complete.wait().await;
        self.is_complete()
    }
}

/// Wake startup waiters if `initialized` returns early or is cancelled.
pub(in super::super) struct WorkspaceStartupGuard<'a>(
    pub(in super::super) &'a WorkspaceScanReadiness,
);

impl Drop for WorkspaceStartupGuard<'_> {
    fn drop(&mut self) {
        self.0.mark_failed();
    }
}

/// A flag raised once, which tasks can wait for.
#[derive(Debug, Default)]
pub(in super::super) struct OneShotSignal {
    raised: std::sync::atomic::AtomicBool,
    notify: tokio::sync::Notify,
}

impl OneShotSignal {
    pub(in super::super) fn raise(&self) {
        self.raised
            .store(true, std::sync::atomic::Ordering::Release);
        self.notify.notify_waiters();
    }

    fn is_raised(&self) -> bool {
        self.raised.load(std::sync::atomic::Ordering::Acquire)
    }

    pub(in super::super) async fn wait(&self) {
        loop {
            let raised = self.notify.notified();
            tokio::pin!(raised);
            // Register before re-reading, so a raise landing between the
            // check and the await cannot be missed.
            raised.as_mut().enable();
            if self.is_raised() {
                return;
            }
            raised.await;
        }
    }
}

impl Backend {
    /// Wait for the workspace index to reflect a completed folder
    /// scan before answering a workspace-wide query.
    ///
    /// Covers both halves of the startup race: the readiness signal covers
    /// "the scan has not begun yet" (`initialized` pulls config and registers
    /// watchers, two client round-trips, before it scans), and the gate covers
    /// "a scan is in flight" — including a later folder-/config-change
    /// rebuild.  The gate is acquired and released, never held while the
    /// caller answers, so a query cannot stall a scan.
    ///
    /// A session with no workspace folders has no scan worth waiting for, so
    /// it returns immediately rather than burning the budget.
    pub(in super::super) async fn wait_for_workspace_scan(&self) -> jsonrpc::Result<()> {
        if !self.workspace_scan_ready.is_complete() && self.workspace_folder_urls().await.is_empty()
        {
            return Ok(());
        }
        crate::transport_liveness::await_outside_request_admission(async {
            tokio::select! {
                biased;
                () = self.workspace_scan_ready.shutdown.wait() => Err(jsonrpc::Error {
                    code: jsonrpc::ErrorCode::InternalError,
                    message: "Workspace scan stopped during shutdown".into(),
                    data: None,
                }),
                result = async {
                    if !self.workspace_scan_ready.wait().await {
                        return Err(jsonrpc::Error {
                            code: jsonrpc::ErrorCode::InternalError,
                            message: "Workspace startup scan failed".into(),
                            data: None,
                        });
                    }
                    drop(self.workspace_scan_gate.lock().await);
                    Ok(())
                } => result,
            }
        })
        .await
    }

    pub(in super::super) async fn scan_workspace_folders(&self) -> DocumentSourceResult<()> {
        // Held for the whole scan so `ensure_autoload_indexed` can wait out
        // an in-flight scan instead of racing it — see the field doc on
        // `workspace_scan_gate`.
        workspace_scan_trace!("scan.gate.wait");
        let _scan_guard = self.workspace_scan_gate.lock().await;
        workspace_scan_trace!("scan.gate.acquired");
        self.workspace_index
            .write()
            .await
            .invalidate_source_inventory_scan();
        let scan_started = crate::rt::Instant::now();
        let folders = self.workspace_folder_urls().await;
        let roots: Vec<PathBuf> = Self::scan_folder_roots(&folders);
        let roots_count = roots.len();
        workspace_scan_trace!("scan.roots roots={roots:?}");
        // Capture the host's actual source-store census before scanning. Any
        // byte mutation while the index is assembled makes the receipt stale;
        // the final rename gate rechecks it against the live store.
        let source_inventory_provider = self.store.workspace_source_inventory_provider();
        let source_inventory_observations = roots
            .iter()
            .map(|root| self.store.workspace_source_inventory(root))
            .collect::<Vec<_>>();
        // Note: we deliberately do NOT early-return when `roots.is_empty()`.
        // The package resolver must still be (re)built from the editor library
        // paths + `TCLLIBPATH` + discovered installations so single-file /
        // no-folder sessions resolve `package require` against the user's picked
        // "Select Tcl Installation" library paths.  Only the per-folder
        // workspace *indexing* (collecting/analysing on-disk files) is skipped
        // below when there are no roots.
        // Snapshot the dialect-resolution inputs and the set of
        // open documents so the blocking worker can run without
        // touching any async mutex.
        let open: HashSet<Uri> = self.scan_open_document_uris().await;
        let package_inputs_epoch = {
            let analyser_inputs_guard = self.analyser_inputs_gate.read().await;
            let epoch = self.diag_inputs_epoch();
            drop(analyser_inputs_guard);
            epoch
        };
        workspace_scan_trace!("scan.package.begin open={}", open.len());
        let (resolver, files, mut scan_complete) =
            self.build_package_db_and_candidates(roots).await;
        workspace_scan_trace!(
            "scan.package.end elapsed_ms={} complete={scan_complete} files={files:?}",
            scan_started.elapsed().as_millis()
        );
        // Package discovery is asynchronous and may outlast a dialect change.
        // Snapshot the inputs only after it completes, under the same gate that
        // serialises analyser configuration mutations.
        let analyser_inputs_guard = self.analyser_inputs_gate.read().await;
        let scan_analyser_inputs_epoch = self.diag_inputs_epoch();
        let package_inputs_current = package_inputs_epoch == scan_analyser_inputs_epoch;
        let document_dialect_overrides = self.document_dialect_override_snapshot().await;
        let folder_dialects = self.folder_dialect_overrides().await;
        let default_dialect = self.session_dialect().await;
        let scan_class_factory_generation = *self.class_factory_generation.read().await;
        drop(analyser_inputs_guard);
        scan_complete &= package_inputs_current;

        // Drop the library files the autoload tier merged under the
        // *previous* package database before this scan's own batches add their
        // fresh entries below — a stale entry must not survive a rescan, and
        // (rare, but possible) a workspace file reachable only via autoload
        // must end up present, not removed, if a later batch re-adds it under
        // its own URI.
        let (_stale_library_uris, _admissions) = self.retire_previous_autoload_rows().await?;

        // Read and analyse the candidate files across a bounded worker
        // pool, merging into `workspace_index` / the salsa `Project` in
        // batches as they complete.
        Self::trace_workspace_scan_stage("scan.merge.begin", scan_started);
        let (resolver, files_count, scan_inputs_current, scan_coverage_complete) = self
            .analyse_and_merge_scanned_files(
                files,
                open,
                ScanAnalysisInputs {
                    analyser_inputs_epoch: scan_analyser_inputs_epoch,
                    class_factory_generation: scan_class_factory_generation,
                    document_dialect_overrides,
                    folder_dialects,
                    default_dialect,
                },
                resolver,
            )
            .await?;

        workspace_scan_trace!(
            "scan.merge.end elapsed_ms={} files={files_count} inputs_current={scan_inputs_current} coverage_complete={scan_coverage_complete}",
            scan_started.elapsed().as_millis()
        );

        // The resolver is a second publication boundary: a later batch may
        // have been accepted under this snapshot, then the configuration may
        // move while analysis is still running. Do not donate those rows' Tcl
        // auto_path facts to a resolver after its source basis is stale.
        let resolver_published = self
            .publish_workspace_scan_resolver_if_current(
                resolver,
                scan_analyser_inputs_epoch,
                scan_class_factory_generation,
                package_inputs_current && scan_inputs_current && scan_coverage_complete,
            )
            .await;
        scan_complete &= resolver_published && scan_coverage_complete;
        workspace_scan_trace!(
            "scan.resolver.published elapsed_ms={} published={resolver_published} files={files_count} inputs_current={scan_inputs_current} coverage_complete={scan_coverage_complete}",
            scan_started.elapsed().as_millis()
        );
        // Re-home sourced documents under their source-site namespaces.
        Self::trace_workspace_scan_stage("scan.rehome.begin", scan_started);
        self.refresh_source_rehoming().await?;
        Self::trace_workspace_scan_stage("scan.rehome.end", scan_started);
        // Warm the deep salsa tier for documents already open so their first
        // hover / semantic-tokens / diagnostics request is a cache hit;
        // unopened files stay at the
        // lightweight tier (`workspace_index` + the salsa `SourceFile` inputs
        // just batched in, which answer `file_decls` / `item_sigs` on demand)
        // until a request actually needs their deep analysis.
        self.spawn_workspace_warm();
        // Publish the workspace's class-factory oracle *before* the readiness
        // signal. A document opened after the scan must be analysed with it on
        // its very first pass, or its outline and navigation come back empty
        // for every class a cross-file metaclass manufactures until some later
        // edit happens to re-run the diagnostics worker.
        let scan_handles = self.workspace_scan_evidence_handles();
        let _factory_sync = Self::publish_scan_factory_views(&scan_handles, scan_started).await;
        Self::trace_workspace_scan_stage("scan.inventory.begin", scan_started);
        let inventory_class_factory_generation = scan_class_factory_generation;
        self.publish_workspace_scan_inventory_if_current(
            source_inventory_provider,
            source_inventory_observations,
            scan_complete,
            scan_analyser_inputs_epoch,
            inventory_class_factory_generation,
        )
        .await;
        Self::trace_workspace_scan_stage("scan.inventory.end", scan_started);
        // In-process readiness signal, released before the log line so a
        // handler waiting on it is not held up by a client round-trip.
        self.complete_workspace_scan(scan_started, roots_count, files_count)
            .await;

        Ok(())
    }

    /// Builds the package database and walks the workspace trees for
    /// candidate paths, as the first half of [`Self::scan_workspace_folders`].
    ///
    /// Cheap directory-metadata work, so it stays a single blocking call; the
    /// expensive per-file parse-and-analyse happens afterwards, in the
    /// parallelised second half
    /// ([`Self::analyse_and_merge_scanned_files`]). Running every file's
    /// `Analyser::analyse` here instead would serialise the whole startup
    /// scan behind one thread.
    ///
    /// Both halves read through the [`vfs::SourceStore`], so a host that
    /// supplies bytes rather than a filesystem gets the same database and the
    /// same candidate set. A failure folds to an empty pair.
    async fn build_package_db_and_candidates(
        &self,
        roots: Vec<PathBuf>,
    ) -> (PackageResolver, Vec<PathBuf>, bool) {
        // Inputs for the package-database `auto_path`: the editor's
        // `tclLsp.libraryPaths` (config.ini / .tcl-lsp.ini layers are read from
        // the store in the worker) and the discovered-installation cache.
        // Per-folder `libraryPaths` overrides are unioned in so a folder's
        // configured package directories contribute to the shared package
        // database (the database is additive — more known packages only refines
        // W120).
        let mut editor_library_paths = self.editor_library_paths.lock().await.clone();
        {
            let folder_configs = self.folder_configs.lock().await;
            for (_, fc) in folder_configs.iter() {
                if let Some(paths) = &fc.library_paths {
                    for p in paths {
                        if !editor_library_paths.contains(p) {
                            editor_library_paths.push(p.clone());
                        }
                    }
                }
            }
        }
        let discovered_cell = Arc::clone(&self.discovered_tcl);
        let scan_store = Arc::clone(&self.store);
        // `tclLsp.workspaceScan.maxFiles` — the whole-session budget the walk
        // stops at (shared across roots, so it bounds the scan, not each
        // folder).  Read here rather than in the worker: the worker holds no
        // async mutex by design.
        let max_files = *self.workspace_scan_max_files.lock().await;
        crate::rt::spawn_blocking(move || {
            workspace_scan_trace!("discovery.begin");
            let discovered = discovered_cell.get_or_init(|| {
                core_tcl_install::discover(&core_tcl_install::default_search_bases())
            });
            workspace_scan_trace!(
                "discovery.end installations={discovered:?} editor_paths={editor_library_paths:?}"
            );
            let resolver = build_package_resolver(
                scan_store.as_ref(),
                &roots,
                &editor_library_paths,
                discovered,
                WORKSPACE_SCAN_DIR_CAP,
            );
            let mut files: Vec<PathBuf> = Vec::new();
            let mut complete = true;
            for root in &roots {
                workspace_scan_trace!("candidates.begin root={root:?} max_files={max_files}");
                complete &= collect_tcl_files(scan_store.as_ref(), root, max_files, &mut files);
                workspace_scan_trace!(
                    "candidates.end root={root:?} complete={complete} count={}",
                    files.len()
                );
            }
            (resolver, files, complete)
        })
        .await
        .unwrap_or_else(|_| (PackageResolver::new(), Vec::new(), false))
    }

    /// Publish the scan receipt only when its configured analysis inputs are
    /// still current. Acquire the index first, then try the config read lock so
    /// no config writer is awaited while the index is held.
    pub(in super::super) async fn publish_workspace_scan_inventory_if_current(
        &self,
        provider: vfs::WorkspaceSourceInventoryProvider,
        observations: Vec<vfs::WorkspaceSourceInventoryObservation>,
        scan_complete: bool,
        expected_analyser_inputs_epoch: u64,
        expected_class_factory_generation: u64,
    ) -> bool {
        let mut index = self.workspace_index.write().await;
        let Ok(analyser_inputs_guard) = self.analyser_inputs_gate.try_read() else {
            index.set_source_inventory_snapshot_with_scan_status(provider, observations, false);
            return false;
        };
        let Ok(class_factory_guard) = self.class_factory_generation.try_read() else {
            index.set_source_inventory_snapshot_with_scan_status(provider, observations, false);
            return false;
        };
        let current = self.diag_inputs_epoch() == expected_analyser_inputs_epoch
            && *class_factory_guard == expected_class_factory_generation;
        let complete = scan_complete && current;
        index.set_source_inventory_snapshot_with_scan_status(provider, observations, complete);
        drop(class_factory_guard);
        drop(analyser_inputs_guard);
        complete
    }

    /// Publish the current resolver only while the exact inputs that selected
    /// its per-file `auto_path` rows still hold at this final boundary.
    pub(in super::super) async fn publish_workspace_scan_resolver_if_current(
        &self,
        resolver: PackageResolver,
        expected_analyser_inputs_epoch: u64,
        expected_class_factory_generation: u64,
        package_inputs_current: bool,
    ) -> bool {
        let mut published = self.package_resolver.write().await;
        if !package_inputs_current {
            return false;
        }
        let Ok(analyser_inputs_guard) = self.analyser_inputs_gate.try_read() else {
            return false;
        };
        if self.diag_inputs_epoch() != expected_analyser_inputs_epoch {
            return false;
        }
        let Ok(class_factory_guard) = self.class_factory_generation.try_read() else {
            return false;
        };
        if *class_factory_guard != expected_class_factory_generation {
            return false;
        }
        *published = resolver;
        drop(class_factory_guard);
        drop(analyser_inputs_guard);
        true
    }

    fn workspace_scan_workers() -> (
        Arc<Semaphore>,
        WorkspaceScanTaskWindow<WorkspaceScanCandidate>,
    ) {
        let concurrency =
            crate::rt::available_parallelism().min(WORKSPACE_ANALYSIS_MAX_CONCURRENCY);
        let permits = Arc::new(Semaphore::new(concurrency));
        let tasks = WorkspaceScanTaskWindow::new(concurrency);
        (permits, tasks)
    }

    fn workspace_scan_inputs_are_current(
        &self,
        scan_analyser_inputs_epoch: u64,
        scan_class_factory_generation: u64,
    ) -> bool {
        self.analyser_inputs_gate.try_read().is_ok_and(|guard| {
            let current = self.diag_inputs_epoch() == scan_analyser_inputs_epoch;
            drop(guard);
            current
        }) && self
            .class_factory_generation
            .try_read()
            .is_ok_and(|generation| *generation == scan_class_factory_generation)
    }

    /// Reads and analyses `files` across a bounded worker pool, as the
    /// second half of [`Self::scan_workspace_folders`] (the `spawn_workspace_warm`
    /// semaphore pattern — acquire a permit before spawning, so at most
    /// `WORKSPACE_ANALYSIS_MAX_CONCURRENCY` files are being read+analysed at
    /// once), merging into `workspace_index` / the salsa `Project` in batches
    /// as they complete rather than collecting every `AnalysisResult` for the
    /// whole tree before merging any of it — bounding how much of the scan's
    /// own output is ever live in memory at once on a large workspace. `files`
    /// already open in the editor are skipped (their live buffer must not be
    /// clobbered by the on-disk copy); an unreadable path is dropped.
    ///
    /// Returns `resolver` extended with every batch's own `auto_path`
    /// contribution, plus the number of files actually analysed.
    pub(in super::super) async fn analyse_and_merge_scanned_files(
        &self,
        files: Vec<PathBuf>,
        open: HashSet<Uri>,
        inputs: ScanAnalysisInputs,
        mut resolver: PackageResolver,
    ) -> DocumentSourceResult<(PackageResolver, usize, bool, bool)> {
        let ScanAnalysisInputs {
            analyser_inputs_epoch: scan_analyser_inputs_epoch,
            class_factory_generation: scan_class_factory_generation,
            document_dialect_overrides,
            folder_dialects,
            default_dialect,
        } = inputs;
        let open = Arc::new(open);
        let folder_dialects = Arc::new(folder_dialects);
        let (permits, mut tasks) = Self::workspace_scan_workers();
        let mut coverage_complete = true;
        let mut files = files.into_iter();
        let mut files_count = 0usize;
        let mut batch = Vec::new();
        loop {
            while tasks.has_capacity() {
                let Some(path) = files.next() else { break };
                let Some(constructed) = constructed_file_uri(&path) else {
                    coverage_complete = false;
                    continue;
                };
                let uri = constructed.uri.clone();
                let admission = Arc::new(constructed.admission(self.store.as_ref())?);
                let driver = self.fresh_analysis_driver(admission).await;
                let Ok(permit) = Arc::clone(&permits).acquire_owned().await else {
                    coverage_complete = false;
                    files = Vec::new().into_iter();
                    break;
                };
                let open = Arc::clone(&open);
                let document_dialect_overrides = Arc::clone(&document_dialect_overrides);
                let folder_dialects = Arc::clone(&folder_dialects);
                let default_dialect = default_dialect.clone();
                let store = Arc::clone(&self.store);
                Self::enqueue_workspace_scan_candidate(
                    &mut tasks,
                    WorkspaceScanWorker {
                        permit,
                        open,
                        uri,
                        driver,
                        store,
                        path,
                        scan_analyser_inputs_epoch,
                        scan_class_factory_generation,
                        document_dialect_overrides,
                        folder_dialects,
                        default_dialect,
                    },
                );
            }
            let Some(result) = tasks.join_next().await else {
                break;
            };
            let item = result.unwrap_or_else(|error| Self::unavailable_scan_worker(&error));
            if !Self::append_workspace_scan_candidate(item, &mut batch, &mut coverage_complete) {
                continue;
            }
            if batch.len() >= SCAN_MERGE_BATCH_SIZE {
                let PublishedWorkspaceScanBatch {
                    batch: _published_batch,
                    published: _published,
                    rows: published_rows,
                } = self
                    .publish_workspace_scan_batch(
                        std::mem::take(&mut batch),
                        &mut coverage_complete,
                    )
                    .await?;
                files_count += published_rows.len();
                // Only source rows that passed the final currentness commit
                // may contribute their `auto_path` mutations to the resolver.
                resolver = extend_resolver_with_document_auto_paths(
                    self.store.as_ref(),
                    resolver,
                    &published_rows,
                );
            }
        }
        if !batch.is_empty() {
            let PublishedWorkspaceScanBatch {
                batch: _published_batch,
                published: _published,
                rows: published_rows,
            } = self
                .publish_workspace_scan_batch(batch, &mut coverage_complete)
                .await?;
            files_count += published_rows.len();
            resolver = extend_resolver_with_document_auto_paths(
                self.store.as_ref(),
                resolver,
                &published_rows,
            );
        }
        let inputs_current = self.workspace_scan_inputs_are_current(
            scan_analyser_inputs_epoch,
            scan_class_factory_generation,
        );
        Ok((resolver, files_count, inputs_current, coverage_complete))
    }

    /// Merge disk-backed workspace scan results into the shared index **and** the
    /// salsa `Project`, so cross-file diagnostics resolve against the whole
    /// workspace (not just open documents), matching the other cross-document
    /// features.
    ///
    /// The blocking scan snapshots open documents before it starts, but
    /// an editor can open a file while the scan is still running. Recheck
    /// the live document map at publication time so stale on-disk
    /// analysis cannot overwrite the open-buffer entry in either store.
    #[cfg(test)]
    pub(in super::super) async fn merge_workspace_scan_results(
        &self,
        analysed: &[(Uri, String, String, AnalysisResult)],
    ) -> DocumentSourceResult<()> {
        self.publish_disk_results(
            analysed,
            &[],
            DiskRemovalPolicy::ClosedOnly,
            "merge_workspace_scan_results",
        )
        .await?;

        Ok(())
    }
}

impl Backend {
    fn scan_folder_roots(folders: &[Uri]) -> Vec<PathBuf> {
        folders
            .iter()
            .filter_map(|f| f.document_path().map(std::borrow::Cow::into_owned))
            .collect()
    }

    async fn scan_open_document_uris(&self) -> HashSet<Uri> {
        let documents = self.documents.lock("scan_workspace_folders").await;
        let open = documents.keys().cloned().collect();
        drop(documents);
        open
    }

    async fn retire_previous_autoload_rows(
        &self,
    ) -> DocumentSourceResult<(Vec<String>, Vec<DocumentPathAdmission>)> {
        let stale_library_uris = self
            .autoloaded_library_uris
            .lock()
            .await
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let admissions = stale_library_uris
            .iter()
            .map(|raw| {
                let uri = Uri::from_str(raw).map_err(|_| {
                    DocumentSourceUnavailable::InvalidStoredUri {
                        original: raw.clone(),
                    }
                })?;
                DocumentPathAdmission::capture(&uri, self.store.as_ref())
            })
            .collect::<DocumentSourceResult<Vec<_>>>()?;
        if !stale_library_uris.is_empty() {
            let _rehoming_guard = self.rehoming_gate.lock().await;
            let mut index = self.workspace_index.write().await;
            let mut seeds = self.rehomed_source_seeds.lock().await;
            let mut autoloaded = self.autoloaded_library_uris.lock().await;
            let retained = admissions.iter().collect::<Vec<_>>();
            DocumentPathAdmission::with_current(&retained, self.store.as_ref(), || {
                for uri in &stale_library_uris {
                    index.remove_document(uri);
                    seeds.remove(uri);
                }
                for uri in &stale_library_uris {
                    autoloaded.remove(uri);
                }
            })?;
        }

        Ok((stale_library_uris, admissions))
    }

    fn workspace_scan_evidence_handles(&self) -> EvidenceHandles {
        EvidenceHandles {
            db_config: Arc::clone(&self.db_config),
            folder_db_configs: Arc::clone(&self.folder_db_configs),
            db: Arc::clone(&self.db),
            db_files: Arc::clone(&self.db_files),
            db_project_members: Arc::clone(&self.db_project_members),
            db_project: Arc::clone(&self.db_project),
            workspace_index: Arc::clone(&self.workspace_index),
            documents: Arc::clone(&self.documents),
            rehoming_gate: Arc::clone(&self.rehoming_gate),
            live_publication_gate: Arc::clone(&self.live_publication_gate),
            class_factory_generation: Arc::clone(&self.class_factory_generation),
        }
    }

    async fn publish_scan_factory_views(
        handles: &EvidenceHandles,
        scan_started: crate::rt::Instant,
    ) -> ClassFactorySync {
        workspace_scan_trace!(
            "scan.factory.begin elapsed_ms={}",
            scan_started.elapsed().as_millis()
        );
        let factory_sync = sync_workspace_class_factories(handles, None).await;
        workspace_scan_trace!(
            "scan.factory.end elapsed_ms={} moved={} affected={}",
            scan_started.elapsed().as_millis(),
            factory_sync.moved.len(),
            factory_sync.affected_names.len()
        );
        // …then re-index the unopened documents that oracle can change
        // Must follow the publish: the classes a cross-file
        // metaclass manufactures only exist in an analysis that carried the
        // oracle, and the scan's own pass could not have.
        workspace_scan_trace!(
            "scan.factory_reindex.begin elapsed_ms={}",
            scan_started.elapsed().as_millis()
        );
        reindex_unopened_factory_consumers(handles, &factory_sync.affected_names).await;
        workspace_scan_trace!(
            "scan.factory_reindex.end elapsed_ms={}",
            scan_started.elapsed().as_millis()
        );
        // The subclass-provided-method view, published once the
        // scan's index is complete so the first document opened is analysed
        // with the workspace's subclass evidence already in place.  No peers
        // to wake: nothing has published diagnostics yet.
        workspace_scan_trace!(
            "scan.subclass.begin elapsed_ms={}",
            scan_started.elapsed().as_millis()
        );
        sync_workspace_subclass_methods(handles).await;
        sync_workspace_declared_commands(handles).await;
        sync_workspace_source_modules(handles).await;
        workspace_scan_trace!(
            "scan.subclass.end elapsed_ms={}",
            scan_started.elapsed().as_millis()
        );
        factory_sync
    }

    async fn complete_workspace_scan(
        &self,
        scan_started: crate::rt::Instant,
        roots_count: usize,
        files_count: usize,
    ) {
        // The first published package database and workspace index are
        // diagnostics inputs: reports computed before them are stale.
        if !self.workspace_scan_ready.is_complete() {
            self.invalidate_diag_inputs();
        }
        self.workspace_scan_ready.mark_complete();
        workspace_scan_trace!(
            "scan.ready elapsed_ms={}",
            scan_started.elapsed().as_millis()
        );
        // Readiness signal for `package_resolver` / `workspace_index` having
        // just been (re)built from disk — a client (or a test) that needs to
        // know the autoload / cross-file workspace state is current rather
        // than racing this scan should wait on this line instead of an
        // unrelated per-document signal.
        let elapsed_ms = scan_started.elapsed().as_secs_f64() * 1000.0;
        self.client
            .log_message(
                MessageType::LOG,
                format!(
                    "[timing] workspace_folders_scan {elapsed_ms:.0}ms \
                     (roots={roots_count}, files={files_count})"
                ),
            )
            .await;
    }

    fn trace_workspace_scan_stage(stage: &str, scan_started: crate::rt::Instant) {
        workspace_scan_trace!("{stage} elapsed_ms={}", scan_started.elapsed().as_millis());
    }
}
