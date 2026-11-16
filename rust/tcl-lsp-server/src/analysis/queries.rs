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

//! Memoised reads of one document from the query database.

use super::super::Backend;
use super::super::configuration::longest_folder_match;
use super::super::tracked_locks::cancellable_catch;
use std::sync::Arc;
use tower_lsp_server::ls_types::Uri;

impl Backend {
    /// Run the salsa `document_symbols` query for `uri` on a worker thread,
    /// reading the current `SourceFile` input.  Returns `None` when the input
    /// is absent or a concurrent edit cancels the read, so the caller can fall
    /// back to a direct computation.
    ///
    /// Yields the **core** symbol tree, and the caller lifts it into
    /// `lsp_types` afterwards.  Keeping the projection out of the closure is a
    /// liveness requirement, not a style choice: everything inside
    /// `Cancelled::catch` runs while this read handle is open, `set_text` waits
    /// on open reads while holding the server's db mutex, and a lift is a pure
    /// allocation walk with no cancellation checkpoint.  Projecting in here
    /// stalls the next keystroke and — via that mutex — every other request.
    /// See `docs/design/rust/lsp-performance.md`.
    pub(in super::super) async fn db_document_symbols(
        &self,
        uri: &Uri,
    ) -> Option<Vec<tcl_lsp_core::document_symbols::DocumentSymbol>> {
        let file = (*self.db_files.lock().await).get(uri).copied()?;
        let config = self.resolved_db_config(uri).await;
        let snapshot = self.db.snapshot("db_document_symbols").await;
        crate::rt::spawn_blocking(move || {
            salsa::Cancelled::catch(|| tcl_lsp_db::document_symbols(&*snapshot, file, config)).ok()
        })
        .await
        .ok()
        .flatten()
    }

    /// Run the salsa `folding_ranges` query for `uri` on a worker thread,
    /// reading the current `SourceFile` input. Returns `None` when the input
    /// is absent or a concurrent edit cancels the read, so the caller can
    /// fall back to a direct computation. Mirrors
    /// [`Self::db_document_symbols`]; the BIG-IP dialect has no salsa query
    /// (it folds on the stanza tree, not the Tcl analyser) and so never
    /// calls this.
    /// Like [`Self::db_document_symbols`], yields the core ranges and leaves the
    /// lift to the caller, for the same liveness reason.
    pub(in super::super) async fn db_folding_ranges(
        &self,
        uri: &Uri,
    ) -> Option<Vec<tcl_lsp_core::folding::FoldingRange>> {
        let file = (*self.db_files.lock().await).get(uri).copied()?;
        let config = self.resolved_db_config(uri).await;
        let snapshot = self.db.snapshot("db_folding_ranges").await;
        crate::rt::spawn_blocking(move || {
            salsa::Cancelled::catch(|| tcl_lsp_db::folding_ranges(&*snapshot, file, config)).ok()
        })
        .await
        .ok()
        .flatten()
    }

    /// Resolve the salsa `AnalyserConfig` handle for `uri`, mirroring
    /// [`DiagInputs::capture_job`]: a folder that overrides the disabled-codes
    /// set / non-ASCII mode (and so has its own handle in `folder_db_configs`)
    /// wins by longest-prefix match; otherwise the process-global
    /// [`Self::db_config`].  Using the *same* handle the diagnostics path uses
    /// keeps the on-demand feature analyses (hover / definition / references /
    /// completion / code-actions) consistent with the published squiggles in a
    /// multi-root workspace.
    pub(in super::super) async fn resolved_db_config(
        &self,
        uri: &Uri,
    ) -> tcl_lsp_db::AnalyserConfig {
        let folder = self.folder_db_configs.lock().await;
        match longest_folder_match(&folder, uri) {
            Some(cfg) => *cfg,
            None => *self.db_config.lock().await,
        }
    }

    /// Run the salsa `file_analysis_incremental` query for `uri` on a worker
    /// thread, reading the current `SourceFile` input.  Returns `None` when the
    /// input is absent or a concurrent edit cancels the read.  The returned
    /// `Arc` shares the memoised analysis (no deep clone of `AnalysisResult`).
    ///
    /// Uses the incremental, per-item-memoised, cancellable query (not the
    /// coarse `file_analysis`) so every caller of this general-purpose
    /// accessor — hover, completion, `semantic_tokens_range`'s fallback, and
    /// anything else routed through [`Backend::cached_analysis`] —
    /// shares the diagnostics worker's already-computed analysis for this
    /// revision instead of paying for an independent whole-file walk.
    async fn db_file_analysis(
        &self,
        uri: &Uri,
    ) -> Option<Arc<tcl_compiler::analyser::AnalysisResult>> {
        // Await the `JoinHandle` variant so the spawn-and-catch lives in one
        // place; callers that need to race it against a budget take the handle
        // directly (see [`db_file_analysis_handle`]).
        self.db_file_analysis_handle(uri)
            .await?
            .await
            .ok()
            .flatten()
    }

    /// [`db_file_analysis`], but returns the worker `JoinHandle` immediately
    /// instead of awaiting it — so `semantic_tokens_range` can race the read
    /// against the fast-path budget and, on timeout, keep awaiting it from a
    /// detached convergence continuation rather than dropping it and
    /// losing the enriched result.  Same cancellable per-item query and the same
    /// liveness invariant as [`db_semantic_tokens`] (the read is unwound at a
    /// per-item boundary by a concurrent `set_text`).  `None` when there is no
    /// salsa input for `uri`.
    pub(in super::super) async fn db_file_analysis_handle(
        &self,
        uri: &Uri,
    ) -> Option<crate::rt::JoinHandle<Option<Arc<tcl_compiler::analyser::AnalysisResult>>>> {
        let sidecar_revision = self
            .fresh_analysis_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .published_revision()?;
        let file = (*self.db_files.lock().await).get(uri).copied()?;
        let config = self.resolved_db_config(uri).await;
        let snapshot = self.db.snapshot("db_file_analysis_handle").await;
        Some(crate::rt::spawn_blocking(move || {
            cancellable_catch(&snapshot, || {
                tcl_lsp_db::file_analysis_incremental(&*snapshot, file, config)
            })
            .ok()
            .filter(|_| sidecar_revision.is_published_current())
        }))
    }

    /// The document's memoised [`CompilationUnit`] under the pack overlay of
    /// the config [`Self::resolved_db_config`] resolves for `uri`, returned as
    /// the worker `JoinHandle` immediately (see [`db_file_analysis_handle`] for
    /// why the range-convergence path needs it).  `None` when there is no
    /// salsa input for `uri`.
    ///
    /// The overlay is what keeps the viewport's enriched tier in agreement
    /// with the registry it resolves against: `semantic_tokens_range` resolves
    /// through [`Self::registry_for_dialect`], which carries the workspace's
    /// packs, and the full-document query reads the overlaid unit too — so a
    /// unit built without it would describe a pack command's variable writes
    /// differently from both.  It is also the diagnostics path's own build, so
    /// a viewport request after the diagnostics pass is a cache hit.
    ///
    /// [`CompilationUnit`]: tcl_compiler::compilation_unit::CompilationUnit
    pub(in super::super) async fn db_compilation_unit_handle(
        &self,
        uri: &Uri,
    ) -> Option<crate::rt::JoinHandle<Option<Arc<tcl_compiler::compilation_unit::CompilationUnit>>>>
    {
        let sidecar_revision = self
            .fresh_analysis_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .published_revision()?;
        let file = (*self.db_files.lock().await).get(uri).copied()?;
        let config = self.resolved_db_config(uri).await;
        let snapshot = self.db.snapshot("db_compilation_unit_handle").await;
        Some(crate::rt::spawn_blocking(move || {
            cancellable_catch(&snapshot, || {
                tcl_lsp_db::document_compilation_unit_for(&*snapshot, file, config)
            })
            .ok()
            // `None` from the query is an abstention: the workspace's packs
            // are not installed, so there is no unit to hand back.
            .flatten()
            .filter(|_| sidecar_revision.is_published_current())
        }))
    }

    /// Kick off the salsa `semantic_tokens` query for `uri` on a worker
    /// thread, returning its `JoinHandle` immediately rather than awaiting it.
    /// Lets a caller race the computation against a deadline (see
    /// [`Backend::semantic_tokens_core_data`]) without losing the ability to
    /// keep waiting on it — the task keeps running (and its result stays
    /// salsa-memoised) whether or not the caller ever polls the handle again.
    /// `None` when there is no salsa input for `uri` (nothing to kick off);
    /// the handle itself resolves to `None` if a concurrent edit cancels the
    /// read.
    ///
    /// **Invariant this task's liveness depends on:** when
    /// `semantic_tokens_core_data` loses the fast-path race, this handle is
    /// awaited from a *detached* `tokio::spawn` task that can run for
    /// seconds on a large cold file, holding an active salsa read-handle the
    /// whole time. Salsa 0.27 blocks a concurrent `set_text` until every
    /// active read-handle is released, and `set_text` runs under the global
    /// `db` mutex — so a stalled read here would stall the write, and the
    /// write holds the lock every other read waits on too. This only stays
    /// live because `semantic_tokens` calls the cancellable, per-item
    /// [`tcl_lsp_db::file_analysis_incremental`] (checked at each proc/method
    /// body boundary), not the coarse whole-file
    /// [`tcl_lsp_db::file_analysis`]: an edit's `set_text` flips the cancel
    /// flag, this detached read unwinds at its next item boundary, and the
    /// write proceeds promptly. Routing this query back through the coarse
    /// `file_analysis` would be worse still: a single cold background token
    /// computation could serialise every subsequent keystroke behind a
    /// whole-file walk. See
    /// `docs/design/rust/lsp-performance.md` §7.
    pub(in super::super) async fn db_semantic_tokens(
        &self,
        uri: &Uri,
    ) -> Option<crate::rt::JoinHandle<Option<tcl_lsp_core::semantic_tokens::SemanticTokens>>> {
        let sidecar_revision = self
            .fresh_analysis_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .published_revision()?;
        let file = (*self.db_files.lock().await).get(uri).copied()?;
        let config = self.resolved_db_config(uri).await;
        // When the workspace has been indexed into a `Project`, resolve object
        // dispatches against the *cross-file* class index so a `$obj method` on
        // a class defined in another file highlights; otherwise fall back to the
        // local (single-file) hierarchy.
        let project = *self.db_project.lock().await;
        let snapshot = self.db.snapshot("db_semantic_tokens").await;
        Some(crate::rt::spawn_blocking(move || {
            cancellable_catch(&snapshot, || {
                match project.filter(|project| project.files(&*snapshot).contains(&file)) {
                    Some(project) => {
                        tcl_lsp_db::semantic_tokens_project_for_inputs(&*snapshot, file, project)
                    }
                    None => tcl_lsp_db::semantic_tokens(&*snapshot, file, config),
                }
            })
            .ok()
            .filter(|_| sidecar_revision.is_published_current())
        }))
    }

    /// Read the memoised `AnalysisResult` for `uri` from the query database, if
    /// the document has a `SourceFile` input.  Returns the shared `Arc` so the
    /// caller can move a refcounted handle into a `spawn_blocking` worker (and
    /// deref it there) rather than paying an O(file) deep copy per feature
    /// request.  `None` when there is no input (the caller analyses fresh) or a
    /// concurrent edit cancelled the read.
    pub(in super::super) async fn cached_analysis(
        &self,
        uri: &Uri,
    ) -> Option<Arc<tcl_compiler::analyser::AnalysisResult>> {
        self.db_file_analysis(uri).await
    }
}
