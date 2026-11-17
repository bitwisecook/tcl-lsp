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

//! Every per-URI cache borrowed as one group, and the retirement of a URI's state.

use super::DocumentSourceResult;
use super::path_admission::DocumentPathAdmission;
use crate::{
    AdmittedDiskBatch, Arc, Backend, BackendState, DiskRemovalPolicy, EnrichedRefreshAsked,
    HashMap, HashSet, Mutex, SemanticTokensCache, Uri, WorkspaceClassAnalysis, vfs,
};

/// Every per-document-URI cache [`Backend`] holds, borrowed as one group.
///
/// Built only by [`Backend::per_uri_caches`] — whose exhaustive destructuring of
/// `Backend` classifies *every* field and so refuses to compile once a new one
/// is added — and consumed only by [`PerUriCaches::forget`], which destructures
/// this struct exhaustively in turn.  Between them, a per-URI cache cannot be
/// wired into one retirement path and forgotten by another — the asymmetry that
/// otherwise leaves one map uncleared behind a rename or a folder removal while
/// every other retirement path clears it.
struct PerUriCaches<'a> {
    /// [`crate::BackendState::autoloaded_library_uris`] — keyed by `uri.as_str()`.
    pub(in super::super) autoloaded_library_uris: &'a Arc<Mutex<HashSet<String>>>,
    /// [`crate::BackendState::rehomed_source_seeds`] — keyed by `uri.as_str()`.
    pub(in super::super) rehomed_source_seeds: &'a Arc<Mutex<HashMap<String, Vec<String>>>>,
    /// [`crate::BackendState::last_semantic_tokens`].
    pub(in super::super) last_semantic_tokens: &'a SemanticTokensCache,
    /// [`crate::BackendState::semantic_tokens_refresh_asked`].
    pub(in super::super) semantic_tokens_refresh_asked: &'a EnrichedRefreshAsked,
    /// [`crate::BackendState::workspace_class_analyses`].
    pub(in super::super) workspace_class_analyses:
        &'a Arc<Mutex<HashMap<Uri, WorkspaceClassAnalysis>>>,
}

impl PerUriCaches<'_> {
    /// Drop every entry these caches hold for `uris`.
    ///
    /// Exhaustively destructured for the same reason
    /// `WorkspaceIndex`'s `DocumentRecords::clear` is: a cache added to the
    /// group does not compile until it is cleared here too.
    ///
    /// Each map is locked on its own, never nested inside another, so this adds
    /// no edge to the server's global lock order.
    pub(in super::super) async fn forget(
        &self,
        uris: &[Uri],
        admissions: &[&DocumentPathAdmission],
        store: &dyn vfs::SourceStore,
    ) -> DocumentSourceResult<()> {
        if uris.is_empty() {
            return Ok(());
        }
        let Self {
            autoloaded_library_uris,
            rehomed_source_seeds,
            last_semantic_tokens,
            semantic_tokens_refresh_asked,
            workspace_class_analyses,
        } = self;
        {
            let mut autoloaded = autoloaded_library_uris.lock().await;
            DocumentPathAdmission::with_current(admissions, store, || {
                for uri in uris {
                    autoloaded.remove(uri.as_str());
                }
            })?;
        }
        {
            let mut seeds = rehomed_source_seeds.lock().await;
            DocumentPathAdmission::with_current(admissions, store, || {
                for uri in uris {
                    seeds.remove(uri.as_str());
                }
            })?;
        }
        {
            let mut tokens = last_semantic_tokens.lock().await;
            DocumentPathAdmission::with_current(admissions, store, || {
                for uri in uris {
                    tokens.remove(uri);
                }
            })?;
        }
        {
            let mut asked = semantic_tokens_refresh_asked.lock().await;
            DocumentPathAdmission::with_current(admissions, store, || {
                for uri in uris {
                    asked.remove(uri);
                }
            })?;
        }
        {
            let mut analyses = workspace_class_analyses.lock().await;
            DocumentPathAdmission::with_current(admissions, store, || {
                for uri in uris {
                    analyses.remove(uri);
                }
            })?;
        }
        Ok(())
    }
}

impl Backend {
    /// Borrow the per-URI caches as one group, classifying every other
    /// `Backend` field on the way past.
    ///
    /// The `let BackendState { … }` below names **every** shared field, so adding one to
    /// `BackendState` is a compile error here until the author has decided whether it
    /// is keyed by a document URI.  That is deliberately stronger than grouping
    /// the caches into an owned nested struct (the `DocumentRecords::clear`
    /// technique this borrows, in `tcl-lsp-core`'s `workspace_index`): a nested
    /// struct only catches a field added *inside* the group, whereas a new
    /// per-URI map is by definition added to `BackendState` first, which is exactly
    /// the case that must not slip through.  Moving the five fields into an
    /// owned struct would also rewrite dozens of unrelated access sites for no
    /// extra safety.
    ///
    /// The classification, by field:
    ///
    /// - **Per-URI caches** (returned here, forgotten wholesale by
    ///   [`Self::forget_uri_states`]): `autoloaded_library_uris`,
    ///   `rehomed_source_seeds`, `last_semantic_tokens`,
    ///   `semantic_tokens_refresh_asked`, `workspace_class_analyses`.
    /// - **Per-URI, but owned by a dedicated path** — each needs more than a map
    ///   removal, so retirement paths call that path explicitly: `documents`
    ///   (the live buffer set; `did_open` / `did_close` own it, and a folder
    ///   removal deliberately leaves open documents alone), `diag_slots`
    ///   ([`Self::evict_diag_slot`] — a slot a worker is draining is that
    ///   worker's liveness record and must survive), `db_files` /
    ///   `db_tombstones` / `db_project_members` ([`Self::db_remove_source`] retires the salsa handle
    ///   rather than dropping it), `pull_diag_cache` / `closed_diag_gen` /
    ///   `closed_diag_order` ([`Self::clear_closed_diagnostics`], which must also
    ///   publish the empty diagnostic set), `semantic_tokens_convergence` (a
    ///   claim released by [`ConvergenceGuard`] on drop), and `workspace_index`
    ///   (URI-keyed, but the cross-document index — removed under its own write
    ///   lock, batched where the caller has several URIs).
    /// - **Keyed by *folder* URI, not document URI** — untouched by a document
    ///   retirement: `workspace_folders`, `folder_dialects`, `folder_configs`,
    ///   `folder_db_configs`, `folder_db_config_tombstones`.
    /// - **Not keyed by a URI at all** — session configuration, gates, salsa
    ///   inputs, and workspace-wide caches: everything else.
    fn per_uri_caches(&self) -> PerUriCaches<'_> {
        let BackendState {
            // The group.
            autoloaded_library_uris,
            rehomed_source_seeds,
            last_semantic_tokens,
            semantic_tokens_refresh_asked,
            workspace_class_analyses,
            // Per-URI, owned by a dedicated path (see the doc comment).
            documents: _,
            diag_slots: _,
            workspace_index: _,
            original_declaration_registry: _,
            db_files: _,
            source_path_admissions: _,
            db_tombstones: _,
            db_project_members: _,
            pull_diag_cache: _,
            closed_diag_gen: _,
            closed_diag_order: _,
            semantic_tokens_convergence: _,
            // Owned by the host session that installed it, not by any one
            // document lifetime — cleared only by the command that set it.
            document_dialect_overrides: _,
            // Owned by the SpecTcl reload path: watch registrations track the
            // configured pack roots, not any document's lifetime.
            external_pack_watch_globs: _,
            // Owned by the SpecTcl reload path too: the claimed-extension
            // snapshot tracks the pack set, not any document's lifetime.
            pack_source_extensions: _,
            // Keyed by folder URI.
            workspace_folders: _,
            folder_dialects: _,
            folder_configs: _,
            folder_db_configs: _,
            folder_db_config_tombstones: _,
            // Not keyed by a URI.
            diagnostics_admission: _,
            client: _,
            diagnostic_publisher: _,
            document_transforms: _,
            default_dialect: _,
            default_dialect_explicit: _,
            warned_session_dialect: _,
            session_dialect_override: _,
            config_reload: _,
            non_ascii_mode: _,
            disabled_diagnostics: _,
            diagnostics_exclude: _,
            policy_layers: _,
            package_resolver: _,
            recovery_names: _,
            workspace_scan_gate: _,
            workspace_scan_ready: _,
            rehoming_gate: _,
            original_source_view_build_gate: _,
            original_source_views: _,
            discovered_tcl: _,
            editor_library_paths: _,
            workspace_scan_max_files: _,
            package_prefer_latest_default: _,
            package_provides: _,
            script_encoding: _,
            extra_commands: _,
            signature_help_disabled_commands: _,
            spec_pack_paths: _,
            workspace_trust: _,
            spec_packs: _,
            spec_pack_reload: _,
            studio_sources: _,
            spec_pack_reload_seq: _,
            bigip_version: _,
            declared_targets: _,
            generic_variable_patterns: _,
            formatting_settings: _,
            feature_toggles: _,
            optimiser_enabled: _,
            diag_inputs_epoch: _,
            // Not keyed by URI for retirement purposes: every hit is checked
            // against its document's full input revision, and the LRU bound
            // retires the rest.
            fresh_analysis_cache: _,
            analyser_inputs_gate: _,
            edit_barrier_stall_reported: _,
            optimiser_profile: _,
            line_length: _,
            style_line_length: _,
            db: _,
            db_project: _,
            db_config: _,
            client_supports_pull_diagnostics: _,
            client_supports_relative_watch_patterns: _,
            conditional_variable_rename_supported: _,
            environment_notice: _,
            class_factory_generation: _,
            semantic_tokens_refresh_pending: _,
            warm_task: _,
            edit_order: _,
            watched_file_change_gate: _,
            disk_publication_gate: _,
            live_publication_advanced: _,
            live_publication_invalidated: _,
            live_publication_generation: _,
            live_publication_uri_generations: _,
            live_publication_gate: _,
            store: _,
        } = self.state.as_ref();
        PerUriCaches {
            autoloaded_library_uris,
            rehomed_source_seeds,
            last_semantic_tokens,
            semantic_tokens_refresh_asked,
            workspace_class_analyses,
        }
    }

    /// Forget every piece of per-URI state keyed on `uris` — the single entry
    /// point every path that retires a URI goes through.
    ///
    /// Covers the [`PerUriCaches`] group plus `diag_slots`, whose eviction is
    /// not a plain map removal.  Callers keep the rest of their retirement
    /// explicit, because the two paths legitimately differ there: the
    /// `workspace_index` entry and the salsa source (batched for a folder
    /// removal, single for a rename) and the published-diagnostics state
    /// (unconditional for a dead path, badge-carrying files only for a folder
    /// removal).
    pub(in super::super) async fn forget_uri_states(
        &self,
        uris: &[Uri],
        admissions: &[Arc<DocumentPathAdmission>],
    ) -> DocumentSourceResult<()> {
        assert_eq!(uris.len(), admissions.len());
        for (uri, admission) in uris.iter().zip(admissions) {
            admission.check_uri(uri)?;
        }
        let retained = admissions.iter().map(Arc::as_ref).collect::<Vec<_>>();
        self.per_uri_caches()
            .forget(uris, &retained, self.store.as_ref())
            .await?;
        for (uri, admission) in uris.iter().zip(admissions) {
            self.evict_diag_slot(uri, admission).await?;
        }
        Ok(())
    }

    /// Retire every trace of a path a `workspace/didRenameFiles` moved away
    /// from.
    ///
    /// The old path no longer exists on disk, so it is treated exactly as
    /// [`LanguageServer::did_change_watched_files`] treats a `DELETED` file:
    /// dropping only the `workspace_index` entry would leave its salsa
    /// `SourceFile` in the `Project`, counting the renamed file's procedures
    /// twice in cross-file resolution and keeping the old URI's whole memo
    /// chain, pull-cache entry, semantic-token baseline and class analysis
    /// alive for the process's life.  The empty publish is what removes the
    /// client's Problems entry for the dead path.
    ///
    /// The per-URI caches go through the shared [`Self::forget_uri_states`]
    /// rather than being cleared by hand here, which is how
    /// `rehomed_source_seeds` and `autoloaded_library_uris` get missed.  A
    /// retained seed record would preserve an obsolete URI association.
    pub(in super::super) async fn retire_renamed_uri(
        &self,
        uri: &Uri,
        admission: Arc<DocumentPathAdmission>,
    ) -> DocumentSourceResult<()> {
        admission.check_for(uri, self.store.as_ref())?;
        let batch = AdmittedDiskBatch {
            removal_admissions: vec![Arc::clone(&admission)],
            ..Default::default()
        };
        self.publish_scanned_disk_results(
            &batch,
            std::slice::from_ref(uri),
            DiskRemovalPolicy::RenamedPath,
            "did_rename_files: old path",
        )
        .await?;
        {
            let _rehoming_guard = self.rehoming_gate.lock().await;
            self.forget_uri_states(std::slice::from_ref(uri), std::slice::from_ref(&admission))
                .await?;
        }
        admission.check_for(uri, self.store.as_ref())?;
        self.clear_closed_diagnostics(uri).await;
        Ok(())
    }
}
