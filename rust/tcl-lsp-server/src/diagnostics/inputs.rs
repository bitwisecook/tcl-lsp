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

//! The owned inputs of a diagnostics run and their capture.

use std::path::PathBuf;
use std::sync::Arc;

use tower_lsp_server::ls_types::Uri;

use super::{DiagCurrency, DiagInputs, DiagJob, DiagToggles, XcToggles};
use crate::configuration::{FolderConfig, longest_folder_match};
use crate::documents::DocumentPublication;
use crate::uri_norm::DocumentPath as _;
use crate::workspace::uri_under_folder;
use crate::{Backend, path_glob, provider_source};

impl DiagInputs {
    /// Capture the document's current, self-consistent diagnostics input (live
    /// buffer + salsa input handle + config).  Reading at drain time — after the
    /// debounce, once a burst's edits have settled — makes the worker robust to
    /// out-of-order edit processing: it always analyses the latest committed
    /// state.  `None` when the document is not open.
    pub(in super::super) async fn capture_job(&self, uri: &Uri) -> Option<DiagJob> {
        let (text, decode_report, dialect, language_id, revision, version) = {
            let docs = self.documents.lock("capture_job").await;
            let doc = docs.get(uri)?;
            // The edit handler makes the new buffer visible before its Salsa
            // source so it can release EditOrder without wedging requests.
            // A worker left over from the preceding revision can drain
            // inside that gap; declining the capture makes it retire,
            // and the successful source commit schedules a fresh worker.
            // Combining this text/revision with the preceding SourceFile
            // would pass the revision-only delivery guard and briefly publish
            // stale diagnostics for the new edit.
            if doc.publication != DocumentPublication::Indexed {
                return None;
            }
            (
                doc.text.clone(),
                doc.decode_report,
                doc.dialect.clone(),
                doc.language_id.clone(),
                doc.revision,
                doc.version,
            )
        };
        let file = self.db_files.lock().await.get(uri).copied();
        // Prefer a folder-scoped analyser config when the document sits under a
        // folder that overrides disabled codes / non-ASCII mode; else the
        // process-global config.
        let config = {
            let folder = self.folder_db_configs.lock().await;
            match longest_folder_match(&folder, uri) {
                Some(cfg) => *cfg,
                None => *self.db_config.lock().await,
            }
        };
        let mut job = DiagJob {
            diagnostic_pass: None,
            text,
            decode_report,
            dialect,
            language_id,
            currency: DiagCurrency::Open(revision),
            version,
            file,
            config,
        };
        job.diagnostic_pass = self.capture_diagnostic_pass(uri, &job).await;
        Some(job)
    }

    /// The folder-scoped salsa [`tcl_lsp_db::AnalyserConfig`] handle for `uri`
    /// (longest matching folder override, else the process-global config) — the
    /// same resolution [`Self::capture_job`] applies, reused for the closed-file
    /// job capture so a closed file honours the same per-folder
    /// disabled-code / non-ASCII settings it did while open.
    pub(super) async fn closed_file_config(&self, uri: &Uri) -> tcl_lsp_db::AnalyserConfig {
        let folder = self.folder_db_configs.lock().await;
        match longest_folder_match(&folder, uri) {
            Some(cfg) => *cfg,
            None => *self.db_config.lock().await,
        }
    }
}

impl Backend {
    /// Run the analyser on `text` and push the resulting
    /// diagnostics to the LSP client for `uri`.  Runs
    /// the analyser on a `spawn_blocking` worker so the LSP
    /// event loop stays responsive.
    /// Gather the document-independent handles a detached diagnostics run needs
    /// (per-edit state travels in a [`DiagJob`]).
    pub(in super::super) async fn diag_inputs(
        &self,
        uri: &Uri,
        dialect: &'static tcl_dialect::DialectProfile,
    ) -> DiagInputs {
        self.diag_inputs_with(uri, dialect, false).await
    }

    pub(in super::super) async fn diag_inputs_with(
        &self,
        uri: &Uri,
        dialect: &'static tcl_dialect::DialectProfile,
        registry_at_drain: bool,
    ) -> DiagInputs {
        let epoch = self.diag_inputs_epoch();
        let (disabled, non_ascii_mode) = self.resolved_analysis_settings(uri).await;
        let policy_layers = self.resolved_policy_layers(uri).await;
        let (pack_generation, pack_set) = {
            let published = self.spec_packs.lock().await;
            (published.seq, Arc::clone(&published.packs))
        };
        let (registry, registry_is_current) = if registry_at_drain {
            (Self::fallback_diagnostic_registry(dialect, &pack_set), true)
        } else {
            self.diagnostic_registry(uri, dialect, &pack_set).await
        };
        let xc_diagnostics = self.xc_diagnostics_enabled(uri).await;
        let cross_file_resolution = self.cross_file_resolution_enabled(uri).await;
        let extra_commands = self
            .resolved_extra_commands(uri)
            .await
            .into_iter()
            .collect();
        let generic_variable_patterns = self.resolved_generic_variable_patterns(uri).await;
        let style_line_length = self.resolved_style_line_length(uri).await;
        let diagnostics_enabled = self.feature_enabled("diagnostics", uri).await;
        let excluded = self.diagnostics_excluded(uri).await;
        let (entry_points, folder_root) = self.w120_inheritance_config(uri).await;
        let data_group_root =
            super::f5_models::data_group_workspace_root(&self.workspace_folder_urls().await, uri);
        let sidecar_revision = self
            .fresh_analysis_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .revision();
        let mut inputs = DiagInputs {
            inputs_epoch: None,
            configuration_epoch: epoch,
            sidecar_revision,
            source_inputs_epoch: Arc::clone(&self.diag_inputs_epoch),
            source_inputs_gate: Arc::clone(&self.analyser_inputs_gate),
            source_path_admissions: Arc::clone(&self.source_path_admissions),
            client: self.client.clone(),
            diagnostic_publisher: Arc::clone(&self.diagnostic_publisher),
            registry,
            registry_at_drain,
            pack_seq: pack_generation,
            pack_set: Arc::clone(&pack_set),
            spec_packs: Arc::clone(&self.spec_packs),
            disabled,
            policy_layers,
            extra_commands,
            generic_variable_patterns,
            style_line_length,
            non_ascii_mode,
            script_encoding: self.declared_script_encoding(),
            documents: Arc::clone(&self.documents),
            diag_slots: Arc::clone(&self.diag_slots),
            workspace_index: Arc::clone(&self.workspace_index),
            rehomed_source_seeds: Arc::clone(&self.rehomed_source_seeds),
            rehoming_gate: Arc::clone(&self.rehoming_gate),
            live_publication_gate: Arc::clone(&self.live_publication_gate),
            package_resolver: Arc::clone(&self.package_resolver),
            package_prefer: self.default_package_prefer().await,
            provider_dialects: provider_source::ProviderDialectSources::of(self),
            recovery_names: Arc::clone(&self.recovery_names),
            entry_points,
            folder_root,
            data_group_root,
            db: Arc::clone(&self.db),
            db_files: Arc::clone(&self.db_files),
            db_project_members: Arc::clone(&self.db_project_members),
            db_project: Arc::clone(&self.db_project),
            class_factory_generation: Arc::clone(&self.class_factory_generation),
            db_config: Arc::clone(&self.db_config),
            folder_db_configs: Arc::clone(&self.folder_db_configs),
            pull_diag_cache: Arc::clone(&self.pull_diag_cache),
            closed_diag_gen: Arc::clone(&self.closed_diag_gen),
            toggles: DiagToggles {
                diagnostics_enabled,
                excluded,
                xc: XcToggles {
                    xc_diagnostics,
                    cross_file_resolution,
                },
            },
            client_supports_pull: self
                .client_supports_pull_diagnostics
                .load(std::sync::atomic::Ordering::Relaxed),
            store: Arc::clone(&self.store),
        };
        let publication_is_current = {
            let current = self.spec_packs.lock().await;
            current.seq == pack_generation && Arc::ptr_eq(&current.packs, &pack_set)
        };
        // A policy-clear receipt retains source and configuration currency
        // without claiming that analysis preparation succeeded.
        if self.diag_inputs_epoch() == epoch
            && publication_is_current
            && (registry_is_current || !diagnostics_enabled || excluded)
        {
            inputs.inputs_epoch = Some(epoch);
        }
        inputs
    }

    /// Whether `uri` matches the resolved `tclLsp.diagnostics.exclude` glob
    /// list. Path patterns are matched against the workspace-folder-
    /// relative path; name patterns against the file name alone, so a
    /// no-folder document can still be excluded by name.
    pub(in super::super) async fn diagnostics_excluded(&self, uri: &Uri) -> bool {
        let (patterns, folder_root) = self.diagnostics_exclude_config(uri).await;
        if patterns.is_empty() {
            return false;
        }
        let Some(path) = uri.document_path() else {
            return false;
        };
        let set = path_glob::PathGlobSet::new(&patterns);
        if set.is_empty() {
            return false;
        }
        let rel = folder_root
            .as_deref()
            .and_then(|root| path.strip_prefix(root).ok());
        set.matches_path(rel, &path)
    }

    /// The exclude-glob list and folder root governing `uri`: the longest
    /// containing workspace folder's `diagnostics.exclude` when its merged
    /// config sets one (that merge already layers global < editor < project),
    /// falling back to the process-global list when the folder sets none —
    /// or when no folder contains `uri` at all, in which case there is no
    /// root and only name patterns can match.
    async fn diagnostics_exclude_config(&self, uri: &Uri) -> (Vec<String>, Option<PathBuf>) {
        let folder = {
            let configs = self.folder_configs.lock().await;
            let mut best: Option<&(Uri, FolderConfig)> = None;
            for entry in configs.iter() {
                if uri_under_folder(uri.as_str(), entry.0.as_str())
                    && best.is_none_or(|b| entry.0.as_str().len() > b.0.as_str().len())
                {
                    best = Some(entry);
                }
            }
            best.map(|(folder, fc)| {
                (
                    fc.diagnostics_exclude.clone(),
                    folder.document_path().map(std::borrow::Cow::into_owned),
                )
            })
        };
        match folder {
            Some((Some(patterns), root)) => (patterns, root),
            Some((None, root)) => (self.diagnostics_exclude.lock().await.clone(), root),
            None => (self.diagnostics_exclude.lock().await.clone(), None),
        }
    }

    /// Retire every cached [`DiagSlot::latest_inputs`], so the next scheduled
    /// run — **including one an ordinary edit schedules** — resolves its
    /// analyser inputs from the configuration as it stands now.
    ///
    /// # The contract
    ///
    /// Call this from every path that writes state [`Self::diag_inputs`] reads:
    /// the feature toggles, the optimiser switch / profile / per-code
    /// overrides, the disabled-diagnostic and severity-override sets, the
    /// non-ASCII mode, `extraCommands`, `genericVariablePatterns`, the style
    /// line length, the diagnostics exclude globs, the per-folder configs, and
    /// the `SpecTcl` pack set (which decides the command registry).  It is a
    /// single atomic increment — cheap enough that erring towards calling it is
    /// always right, and the only cost of a spurious call is one extra
    /// `diag_inputs` resolve on the next schedule.
    ///
    /// `AcqRel` / `Acquire`, matching [`EditOrder`]: the writes this bump
    /// announces are the config mutations that precede it, so a scheduler that
    /// observes the new epoch must also observe them.  Relaxed would let the two
    /// be reordered and reintroduce the window in miniature.
    ///
    /// # Why an epoch and not just `reschedule_all_open_documents`
    ///
    /// The config reload already reschedules every open document under the new
    /// toggles — but only at the *end* of [`Self::run_config_reload`], after the
    /// pack-discovery walk.  The switches themselves are applied at the start,
    /// which is also the instant `tcl-lsp.getEffectiveConfig` starts reporting
    /// them.  A client that (reasonably) treats that command as the settle
    /// signal and then types a character lands inside that window, and the
    /// keystroke's `schedule_diagnostics` would reuse inputs resolved *before*
    /// the apply: the optimiser off in the reported config and still on in the
    /// analysis, so O-codes the user had just disabled come back on that
    /// publish and clear only on the reschedule a moment later.
    ///
    /// Widening the reschedule's reach cannot close that window — the edit
    /// arrives *during* it, so the only thing that can be right is what the
    /// edit itself reads.
    pub(in super::super) fn invalidate_diag_inputs(&self) {
        self.diag_inputs_epoch
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
    }

    /// The current [`Self::diag_inputs_epoch`].
    pub(in super::super) fn diag_inputs_epoch(&self) -> u64 {
        self.diag_inputs_epoch
            .load(std::sync::atomic::Ordering::Acquire)
    }
}
