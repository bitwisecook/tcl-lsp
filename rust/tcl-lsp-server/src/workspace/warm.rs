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

//! The background warm of the memoised analysis for open documents.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{Mutex, Semaphore};
use tower_lsp_server::ls_types::Uri;

use crate::Backend;
use crate::configuration::longest_folder_match;
use crate::documents::DocumentStore;
use crate::tracked_locks::{TrackedMutex, cancellable_catch};

/// Ceiling on how many **open** documents the background workspace warm
/// analyses concurrently.  The warm pre-populates the
/// memoised per-file analysis for already-open documents across the blocking
/// pool so their first hover / semantic-tokens / diagnostics request finds a
/// cache hit instead of a cold `file_analysis_incremental` walk; the cap keeps
/// it from monopolising the blocking pool (or holding too many salsa snapshots
/// at once, which would stall a concurrent edit's `set_text`), and it is
/// clamped to the machine's parallelism so small hosts stay responsive.
pub(in super::super) const WORKSPACE_WARM_MAX_CONCURRENCY: usize = 16;

impl Backend {
    /// Kick off a detached, concurrency-bounded parallel **warm** of the salsa
    /// per-file analysis for every currently **open** document.
    ///
    /// Deep state (`file_analysis_incremental` and everything built on it —
    /// `compilation_unit`) is an
    /// open-document cost, not a workspace one: an on-disk file the editor hasn't
    /// opened only needs to answer cross-file resolution (`workspace_index`,
    /// fed by the scan's own analyser pass, plus the light `file_decls` /
    /// `item_sigs` / `file_token_facts` tier salsa computes on demand from the
    /// `SourceFile`
    /// inputs the disk-publication transaction sets). Warming the deep tier for
    /// every workspace file would analyse the whole project a *second* time
    /// through salsa on top of the scan's own analyser walk, and materialise
    /// `file_analysis` for files nobody has open — hundreds of megabytes of RSS
    /// on a large tree. This warm primes only the files already open when it
    /// runs, so the first hover / semantic-tokens / diagnostics
    /// request on an already-open tab (e.g. several restored editor tabs right
    /// after `initialized`, or a big multi-root reload) is a cache hit instead
    /// of a cold `file_analysis_incremental` walk; an unopened file pays its
    /// deep-tier cost only if a request actually needs it, exactly like a
    /// document opened after the scan already does.
    ///
    /// Purely an optimisation, and safe by construction:
    /// - **Correctness**: it only primes the salsa cache; a concurrent real read
    ///   of the same `(file, config)` dedups against it (salsa blocks the second
    ///   requester and shares the memoised result — never a double analysis or a
    ///   divergent one), so results are identical to an unwarmed cold read.
    /// - **Never blocks a request**: fire-and-forget on a detached task.
    /// - **Never stalls an edit**: at most `WORKSPACE_WARM_MAX_CONCURRENCY`
    ///   snapshots are live at once (each warm clones its snapshot only after
    ///   acquiring a permit and drops it as soon as its analysis returns), so a
    ///   concurrent `set_text` waits on at most that many in-flight reads — and
    ///   each is the cancellable per-item query, so `set_text` cancels them at a
    ///   per-item boundary rather than waiting them out. A fresh warm aborts the
    ///   previous one (`warm_task`), so overlapping scans (initialize / folder-add
    ///   / config-change) keep that bound *global*, not merely per-warm, and drop
    ///   the redundant re-walk.
    ///
    /// Each open document warms under its own [`Self::resolved_db_config`] —
    /// the config the diagnostics / hover / completion path would actually
    /// resolve for it — rather than a full `files × configs` cross product.
    /// Cross-file token indexes use each project's per-file configuration
    /// mapping on their light structure tier. This warm touches only open
    /// documents and shares each document's own deep query.
    pub(in super::super) fn spawn_workspace_warm(&self) {
        let db = Arc::clone(&self.db);
        let db_files = Arc::clone(&self.db_files);
        let documents = Arc::clone(&self.documents);
        let db_config = Arc::clone(&self.db_config);
        let folder_db_configs = Arc::clone(&self.folder_db_configs);
        let task = crate::rt::spawn(Self::warm_open_documents(
            db,
            db_files,
            documents,
            db_config,
            folder_db_configs,
        ));
        // Supersede any still-running warm so overlapping scans (initialize /
        // folder-add / config-change) can't stack their snapshots or redundantly
        // re-walk the open set. Aborting at the per-item await boundaries (with
        // the existing `Cancelled::catch`) keeps the global snapshot bound at
        // `WORKSPACE_WARM_MAX_CONCURRENCY`.
        if let Ok(mut guard) = self.warm_task.lock()
            && let Some(prev) = guard.replace(task.abort_handle())
        {
            prev.abort();
        }
    }

    /// The body [`Self::spawn_workspace_warm`] detaches — pulled out as its own
    /// `async fn` over cloned `Arc` fields (rather than `&self`) so a test can
    /// `.await` it directly and observe, deterministically, exactly which salsa
    /// queries it executes (`TclDatabase::with_event_logger`) instead of racing
    /// a fire-and-forget `tokio::spawn`.
    pub(in super::super) async fn warm_open_documents(
        db: Arc<TrackedMutex<tcl_lsp_db::TclDatabase>>,
        db_files: Arc<TrackedMutex<HashMap<Uri, tcl_lsp_db::SourceFile>>>,
        documents: Arc<DocumentStore>,
        db_config: Arc<Mutex<tcl_lsp_db::AnalyserConfig>>,
        folder_db_configs: Arc<Mutex<Vec<(Uri, tcl_lsp_db::AnalyserConfig)>>>,
    ) {
        let open_uris: Vec<Uri> = documents
            .lock("warm_open_documents")
            .await
            .keys()
            .cloned()
            .collect();
        if open_uris.is_empty() {
            return;
        }
        let global_config = *db_config.lock().await;
        let folder_configs = folder_db_configs.lock().await.clone();
        let work: Vec<(tcl_lsp_db::SourceFile, tcl_lsp_db::AnalyserConfig)> = {
            let files = db_files.lock().await;
            open_uris
                .iter()
                .filter_map(|uri| {
                    let file = *files.get(uri)?;
                    let config = longest_folder_match(&folder_configs, uri)
                        .copied()
                        .unwrap_or(global_config);
                    Some((file, config))
                })
                .collect()
        };
        if work.is_empty() {
            return;
        }
        let concurrency = crate::rt::available_parallelism().min(WORKSPACE_WARM_MAX_CONCURRENCY);
        let permits = Arc::new(Semaphore::new(concurrency));
        let mut warms = crate::rt::JoinSet::new();
        for (file, config) in work {
            // Acquire the permit *before* spawning so at most `concurrency`
            // warm tasks (and thus snapshots) exist at once: the loop
            // backpressures rather than parking one pending task per item on
            // the semaphore. `acquire_owned` moves the permit into the task,
            // which holds it until its analysis returns.
            let Ok(permit) = Arc::clone(&permits).acquire_owned().await else {
                break;
            };
            let db = Arc::clone(&db);
            warms.spawn(async move {
                let _permit = permit;
                // Clone the snapshot only after the permit is held, so
                // `set_text` never waits on more than `concurrency` in-flight
                // reads.
                let snapshot = db.snapshot("warm_open_documents").await;
                let _ = crate::rt::spawn_blocking(move || {
                    let _ = cancellable_catch(&snapshot, || {
                        tcl_lsp_db::file_analysis_incremental(&*snapshot, file, config)
                    });
                })
                .await;
            });
        }
        while warms.join_next().await.is_some() {}
    }
}
