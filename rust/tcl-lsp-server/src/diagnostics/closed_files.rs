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

//! Diagnostics badges of closed files.

use super::run::run_diagnostics_core;
use super::{Arc, Backend, DiagCurrency, DiagJob, Uri};

/// How many **closed** files keep a server-side diagnostics badge record
/// (`pull_diag_cache` + `closed_diag_gen`) before the least-recently-published
/// one is evicted.
///
/// A closed workspace file keeps its Problems badge, so without a bound,
/// browsing a large tree would retain a full `Vec<Diagnostic>` for every file
/// the editor ever opened, for the process's life.  Every entry is re-derivable
/// from disk, so evicting the oldest costs nothing but a recompute on reopen.
/// Sized well above a realistic "recently visited" working set (`VS Code`'s own
/// Problems view is the consumer) yet far below the thousands of files a
/// workspace walk touches.
pub(in super::super) const CLOSED_DIAG_BADGE_CAP: usize = 512;

impl Backend {
    /// Compute and publish a **closed** workspace file's diagnostics from its
    /// on-disk contents, so a file that was opened and had its editor tab
    /// closed keeps its Problems / File-Explorer badge instead of losing it the
    /// moment the tab closes.  Runs the *same* [`run_diagnostics_core`] pipeline
    /// the open path uses — analyser, compiler checks, source-style, and the
    /// cross-file W120/W123 refinement — so a closed file shows exactly the set
    /// it showed while open, kept accurate against disk.
    ///
    /// Callers must have refreshed the on-disk salsa source first (via
    /// [`Self::reindex_index_from_disk_using`]).  When the URI has no readable on-disk
    /// `SourceFile` (an untitled buffer, or a file deleted between the reindex and
    /// here), there is nothing to analyse and the stale squiggles are cleared
    /// instead.  The
    /// dialect is resolved from the on-disk content (matching the `SourceFile`
    /// dialect the reindex stored), and the run captures a fresh per-URI
    /// generation so a newer refresh supersedes it.
    pub(in super::super) async fn publish_closed_file_diagnostics(&self, uri: &Uri) {
        // An open buffer is owned by the open path; never shadow it from disk.
        if self
            .documents
            .lock("publish_closed_file_diagnostics")
            .await
            .contains_key(uri)
        {
            return;
        }
        // The on-disk-backed salsa source `reindex_index_from_disk` primed; its
        // absence means the URI is untitled / deleted, so clear instead.
        let Some(file) = self.db_files.lock().await.get(uri).copied() else {
            self.clear_closed_diagnostics(uri).await;
            return;
        };
        // Read the handle's text under the `db` lock alone (the `db_files` lock
        // is released), preserving the global `db` → `db_files` order.
        let text = {
            let db = self.db.lock().await;
            file.text(&*db).clone()
        };
        // Resolve the dialect from the on-disk source the same way `did_open`
        // does (basename / BIG-IP, in-source `# tcl-dialect:` directive, folder
        // override, session default) rather than folder-or-default alone, so a
        // closed BIG-IP config or a version-pinned Tcl file keeps the dialect it
        // had while open instead of being re-analysed as generic Tcl.
        let dialect = self.dialect_for_closed(uri, &text).await;
        // Closed diagnostics are also backed by the exact on-disk text, so the
        // same byte-evidence contract used at `didOpen` applies here.
        let decode_report = Self::decode_report_for_matching_disk_text(
            &self.store,
            uri,
            &text,
            self.declared_script_encoding(),
        )
        .await;
        // Bump this URI's closed-run generation and capture it, so a newer close
        // / watched-change refresh supersedes this run at publish time.
        let generation = self.next_closed_diag_generation(uri).await;
        let inputs = self
            .diag_inputs(uri, tcl_lsp_core::profile_for_dialect(&dialect))
            .await;
        let config = inputs.closed_file_config(uri).await;
        let job = DiagJob {
            diagnostic_pass: None,
            // A closed file's text comes from the salsa input, not from an open
            // snapshot, so this is where its shared handle is minted.
            text: Arc::from(text),
            decode_report,
            dialect,
            // A closed file carries no editor `language_id`; F5 model dialects are
            // routed by the resolved `dialect` + basename resolved above.
            language_id: String::new(),
            currency: DiagCurrency::ClosedFromDisk(generation),
            version: None,
            // Deliberately **not** `Some(file)`.  Routing a closed file
            // through the salsa queries would memoise its whole deep compiler
            // tier — the `compilation_unit` IR/CFG/SSA build behind
            // `file_analysis_incremental` / `compiler_check_diagnostics` — and
            // salsa cannot evict it, so every file the editor ever opened would
            // keep tens of megabytes of compiler memos for the process's life,
            // which a workspace browse turns into an out-of-memory kill.  This
            // run happens *inline* on every close, so it, not the debounced open
            // path, is what would pin a memo chain for files the user merely
            // glanced at.  The uncached pipeline
            // computes the same per-file diagnostics and drops everything it
            // built, keeping the closed tier bounded; the file's `SourceFile`
            // input stays in the `Project` so its *lightweight* decls/signature
            // queries keep serving open documents' cross-file resolution.
            file: None,
            config,
        };
        run_diagnostics_core(inputs, uri, job).await;
        self.record_closed_diag_badge(uri).await;
    }

    /// Bump and return `uri`'s closed-file diagnostics generation. Each
    /// closed run captures the value this returns; the publish-time currency
    /// guard ([`DeliveryCtx::is_current`]) drops any run whose captured
    /// generation is no longer the latest, so an older run finishing after a
    /// newer one cannot republish stale diagnostics.
    async fn next_closed_diag_generation(&self, uri: &Uri) -> u64 {
        let mut gens = self.closed_diag_gen.lock().await;
        let slot = gens.entry(uri.clone()).or_insert(0);
        *slot = slot.wrapping_add(1);
        *slot
    }

    /// Clear any previously-published diagnostics for a closed URI that no longer
    /// has an on-disk source (untitled / deleted), and drop its pull-cache entry.
    /// Guarded on the document still being closed, so a `did_open` racing in
    /// cannot have this empty publish blank a freshly reopened buffer.
    pub(in super::super) async fn clear_closed_diagnostics(&self, uri: &Uri) {
        let receipt = {
            let docs = self.documents.lock("clear_closed_diagnostics").await;
            if docs.contains_key(uri) {
                return;
            }
            // Commit the clear, pull-cache removal and generation retirement in
            // one document-ordered critical section. The publisher performs the
            // client I/O after this guard is released.
            self.pull_diag_cache.lock().await.remove(uri);
            self.closed_diag_gen.lock().await.remove(uri);
            self.diagnostic_publisher
                .enqueue(uri.clone(), Vec::new(), None, false, false)
        };
        let _ = receipt.await;
        // …and its place in the closed-badge recency list, so a cleared URI does
        // not count against [`CLOSED_DIAG_BADGE_CAP`].
        self.closed_diag_order
            .lock()
            .await
            .retain(|queued| queued != uri);
    }

    /// Record that `uri` now carries a **closed**-file badge and evict the
    /// oldest closed badges beyond [`CLOSED_DIAG_BADGE_CAP`].
    ///
    /// Open documents are never queued here, so they are never evicted.  An
    /// evicted URI only loses the server's *record* of its badge — the client
    /// keeps the diagnostics it was last sent, and reopening the file (or a
    /// watched-file change) recomputes them from disk — so the closed-file
    /// badge survives; what stops growing is the per-URI `Vec<Diagnostic>` the
    /// server would otherwise hold for every file the user ever opened.
    pub(in super::super) async fn record_closed_diag_badge(&self, uri: &Uri) {
        let evicted: Vec<Uri> = {
            let mut order = self.closed_diag_order.lock().await;
            order.retain(|queued| queued != uri);
            order.push_back(uri.clone());
            let mut evicted = Vec::new();
            while order.len() > CLOSED_DIAG_BADGE_CAP {
                if let Some(oldest) = order.pop_front() {
                    evicted.push(oldest);
                }
            }
            evicted
        };
        if evicted.is_empty() {
            return;
        }
        let docs = self.documents.lock("record_closed_diag_badge").await;
        let mut cache = self.pull_diag_cache.lock().await;
        let mut gens = self.closed_diag_gen.lock().await;
        for uri in &evicted {
            if docs.contains_key(uri) {
                continue;
            }
            cache.remove(uri);
            // Same lifetime as the cache entry — the generation only orders
            // runs for a URI the server is still tracking.
            gens.remove(uri);
        }
    }

    /// Refresh the diagnostics of every **closed** file that currently carries a
    /// badge (i.e. has a pull-cache entry but is not open), after a workspace-wide
    /// change (a config / master-switch toggle, a disabled-code change) that the
    /// open-document reschedule does not cover.  A closed file that is now clean,
    /// or whose diagnostics are disabled by the new config, is emptied by the
    /// same pipeline; one that still has problems is republished so its badge
    /// stays accurate.
    pub(in super::super) async fn reschedule_closed_file_diagnostics(&self) {
        // Snapshot the closed-and-badged URIs under `documents` → `pull_diag_cache`
        // (the established lock order) before doing any per-file work.
        let closed: Vec<Uri> = {
            let docs = self
                .documents
                .lock("reschedule_closed_file_diagnostics")
                .await;
            let cache = self.pull_diag_cache.lock().await;
            cache
                .keys()
                .filter(|uri| !docs.contains_key(*uri))
                .cloned()
                .collect()
        };
        for uri in closed {
            self.publish_closed_file_diagnostics(&uri).await;
        }
    }
}
