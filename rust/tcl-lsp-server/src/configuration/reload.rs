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

//! The configuration pull and the coalesced reload a `didChangeConfiguration` burst runs.

use tcl_lsp_core::{config_ini, tcl_install as core_tcl_install};
use tower_lsp_server::ls_types::{ConfigurationItem, MessageType, Uri};

use super::folder_settings::parse_folder_config;
use super::folder_settings::signature_help_disabled_commands;
use super::setting_parsers::{collapse_inlay_alias, normalize_config_payload, read_ini_layer};
use super::{FolderConfig, PolicyLayers};
use crate::Backend;
use crate::documents::DocumentSourceResult;
use crate::protocol::{CLIENT_REQUEST_TIMEOUT, FoldingRangeRefreshRequest, bounded_client_request};
use crate::spec_packs::ReloadTrigger;
use crate::uri_norm::DocumentPath as _;

/// How long a `workspace/didChangeConfiguration` leader waits before running
/// the reload, so the rest of a burst folds into the same generation.
///
/// Long enough to swallow a settings-editor keystroke burst (`VS Code` emits one
/// notification per changed key) and short enough that a single deliberate
/// toggle still feels immediate.
pub(in super::super) const CONFIG_RELOAD_DEBOUNCE: std::time::Duration =
    std::time::Duration::from_millis(150);

/// Coalescing state for the configuration reload (see
/// [`Backend::coalesced_config_reload`]).
///
/// The same leader-and-dirty-flag shape as [`DiagSlot`], and for the same
/// reason: the flag records only *that* another notification arrived, so the
/// run always reads the freshest state rather than a snapshot captured when it
/// was scheduled.
#[derive(Default)]
pub(in super::super) struct ConfigReloadSlot {
    /// A notification arrived that the running reload has not yet covered.
    pub(in super::super) dirty: bool,
    /// A handler is currently acting as the reload leader.
    pub(in super::super) running: bool,
}

impl Backend {
    /// Pull the `tclLsp` configuration section from the client and apply it.
    ///
    /// The editor (and the e2e harness) answer `workspace/configuration` with
    /// the resolved `tclLsp` settings; a bare `didChangeConfiguration` with an
    /// empty payload is the signal to re-pull.  Applies `features.*`, the
    /// optimiser switch, line length, dialect, and the analyser knobs (W108
    /// mode + disabled diagnostics) — only the keys the reply carries, so
    /// omitted keys keep their last-applied value.
    pub(in super::super) async fn pull_and_apply_config(&self) -> DocumentSourceResult<()> {
        self.pull_and_apply_config_values().await?;
        // Whatever the client answered, the setting that decides whether a
        // notice may be shown is applied as far as it will be, so a notice
        // waiting on the first pull can decide.
        self.environment_notice.mark_config_settled();
        // Both applies can move a document's dialect: `apply_global_config`
        // rewrites the session `default_dialect`, and `apply_folder_configs`
        // replaces the per-folder `tclLsp.dialect` map.  Every already-open
        // buffer must be re-resolved against the new state here, at the single
        // point every pull passes through, rather than at each of the four call
        // sites (`initialized`, `did_change_configuration`, the workspace-folder
        // change, and the `.tcl-lsp.ini` watcher).  A call site that skips it
        // leaves a transient session-global dialect baked into open buffers —
        // the pull reverts the global without re-resolving — and leaves a
        // document opened concurrently with `initialized`'s pull stuck on the
        // pre-config default.
        //
        // Outside the pull body on purpose: a client that declines
        // `workspace/configuration` (or answers with an empty array) still needs
        // the re-resolve, because the inline `didChangeConfiguration` payload it
        // *did* send may have changed the session default — the flat MCP-bridge
        // shape carries the dialect that way and nothing else would apply it.
        self.reresolve_open_document_dialects().await?;

        Ok(())
    }

    /// `workspace/configuration`, bounded by [`CLIENT_REQUEST_TIMEOUT`].
    ///
    /// The timeout bounds this request's lifetime and lets later pulls retry.
    /// The transport independently guarantees that a parked handler cannot
    /// stop stdin from routing its eventual reply; see
    /// [`crate::transport_liveness`].
    ///
    /// A timeout is reported as `Err(())`, the same shape as a client that
    /// declines, so every caller's existing early-return path handles it: the
    /// server keeps the config it already had rather than acting on a partial
    /// pull.
    async fn request_configuration(
        &self,
        items: Vec<ConfigurationItem>,
    ) -> Result<Vec<serde_json::Value>, ()> {
        match crate::rt::timeout(CLIENT_REQUEST_TIMEOUT, self.client.configuration(items)).await {
            Ok(Ok(values)) => Ok(values),
            Ok(Err(_)) => Err(()),
            Err(_elapsed) => {
                let message = format!(
                    "tcl-lsp: the editor did not answer workspace/configuration within {}s; \
                     keeping the previous settings so a later pull can retry.",
                    CLIENT_REQUEST_TIMEOUT.as_secs()
                );
                eprintln!("{message}");
                self.client.log_message(MessageType::WARNING, message).await;
                Err(())
            }
        }
    }

    /// The pull-and-apply body of [`Self::pull_and_apply_config`], without the
    /// trailing dialect re-resolve. Returns early when the client declines the
    /// `workspace/configuration` request, or does not answer it in time.
    pub(super) fn pull_and_apply_config_values(
        &self,
    ) -> std::pin::Pin<Box<impl std::future::Future<Output = DocumentSourceResult<()>> + '_>> {
        Box::pin(self.pull_and_apply_config_values_inner())
    }

    async fn pull_and_apply_config_values_inner(&self) -> DocumentSourceResult<()> {
        let items = vec![ConfigurationItem {
            scope_uri: None,
            section: Some("tclLsp".to_owned()),
        }];
        let Ok(values) = self.request_configuration(items).await else {
            return Ok(());
        };
        let Some(cfg) = values.into_iter().next() else {
            return Ok(());
        };
        // Accept nested / flat-dotted / unwrapped payload shapes (some clients,
        // e.g. JetBrains, don't send the nested `tclLsp` object).
        let cfg = normalize_config_payload(&cfg);
        // Layer the editor's pulled
        // `tclLsp` settings between the user `config.ini` `[global]` (lowest)
        // and the project `.tcl-lsp.ini` `[project]` (highest), applying the
        // documented precedence and merge rules.
        let global_ini = read_ini_layer(
            self.store.as_ref(),
            core_tcl_install::user_config_path(),
            config_ini::Layer::Global,
        );
        let folders = self.workspace_folder_urls().await;
        // Session-level apply uses the primary root's project file (the whole
        // story for a single-root workspace; per-folder applies below refine
        // the analyser knobs for multi-root).
        let primary_project = read_ini_layer(
            self.store.as_ref(),
            folders
                .first()
                .and_then(|f| f.document_path().map(std::borrow::Cow::into_owned))
                .map(|root| core_tcl_install::project_config_path(&root)),
            config_ini::Layer::Project,
        );
        // Collapse the legacy `features.inlayHints` alias to `inlayTypeHints`
        // *within each layer* before merging. The alias must be resolved
        // per-layer because `merge_settings` is layer-agnostic: a lower layer
        // (the global `config.ini`) carrying an explicit `inlayTypeHints` would
        // otherwise win over a higher layer (the editor) that only sets the
        // `inlayHints` alias, inverting precedence.
        let mut global_ini = global_ini;
        let mut cfg = cfg;
        let mut primary_project = primary_project;
        collapse_inlay_alias(&mut global_ini);
        collapse_inlay_alias(&mut cfg);
        collapse_inlay_alias(&mut primary_project);
        self.apply_session_layers(&PolicyLayers {
            global: global_ini.clone(),
            editor: cfg,
            project: primary_project.clone(),
        })
        .await?;
        // Per-folder editor configuration: VS Code resolves `tclLsp` settings
        // per scope, so pull each folder's resolved config, layer it between the
        // global `config.ini` and that folder's `.tcl-lsp.ini`, and store it for
        // longest-prefix resolution at read time.  A single-root / no-folder
        // session skips this — the global pull above is the whole story.
        let _: () = if !folders.is_empty() {
            let primary_folder = folders.first().cloned();
            let items: Vec<ConfigurationItem> = folders
                .iter()
                .map(|f| ConfigurationItem {
                    scope_uri: Some(f.clone()),
                    section: Some("tclLsp".to_owned()),
                })
                .collect();
            if let Ok(values) = self.request_configuration(items).await {
                let parsed: Vec<(Uri, FolderConfig)> = folders
                    .into_iter()
                    .zip(values)
                    .filter_map(|(folder, editor_cfg)| {
                        let project = read_ini_layer(
                            self.store.as_ref(),
                            folder
                                .document_path()
                                .map(std::borrow::Cow::into_owned)
                                .map(|root| core_tcl_install::project_config_path(&root)),
                            config_ini::Layer::Project,
                        );
                        let merged = config_ini::merge_settings(
                            &config_ini::merge_settings(&global_ini, &editor_cfg),
                            &project,
                        );
                        parse_folder_config(&merged).map(|mut fc| {
                            let layers = PolicyLayers {
                                global: global_ini.clone(),
                                editor: editor_cfg,
                                project,
                            };
                            // The analyser's skip and the policy come from the
                            // same layers, so the report can explain every gap.
                            fc.disabled_diagnostics = Some(layers.production_skip());
                            fc.policy_layers = Some(layers);
                            (folder, fc)
                        })
                    })
                    .collect();
                self.apply_folder_configs(parsed).await;
            } else {
                // The unscoped pull succeeded, so retain its global/editor
                // layer, but the primary project's higher-precedence INI
                // exclusion must not disappear merely because the scoped pull
                // failed. Patch only that root's override into the last good
                // folder chain; sibling values remain isolated and intact.
                self.apply_primary_project_signature_fallback(
                    primary_folder.as_ref(),
                    &primary_project,
                )
                .await;
            }
        };
        Ok(())
    }

    /// Adopt `layers` as the session's configuration: the diagnostic policy
    /// is built from the layers unmerged — only they can name the layer that
    /// decided a code — and every other setting from their merge, the global
    /// file under the editor's settings under the project file, with
    /// signature suppression's fallback at the global and editor layers
    /// alone. What a configuration pull does for the primary root, after it
    /// has collapsed each layer's inlay alias.
    pub(in super::super) async fn apply_session_layers(
        &self,
        layers: &PolicyLayers,
    ) -> DocumentSourceResult<()> {
        let global_editor = config_ini::merge_settings(&layers.global, &layers.editor);
        let merged = config_ini::merge_settings(&global_editor, &layers.project);
        *self.policy_layers.lock().await = layers.clone();
        self.apply_global_config_with_signature_fallback(&merged, &global_editor)
            .await?;

        Ok(())
    }

    /// Preserve the primary project's signature exclusion when a scoped
    /// configuration pull fails, without replacing any last-known sibling
    /// folder settings.
    ///
    /// Re-applying that last-known chain is also what refreshes everything
    /// those folders *inherit*. A folder handle snapshots each global it does
    /// not override, and the unscoped pull that just succeeded may have moved
    /// them; a chain left untouched here would keep answering
    /// `resolved_db_config` with the old non-ASCII mode, extra commands,
    /// package-provides map, BIG-IP version or declared targets while the
    /// global handle already carries the new ones. So the re-apply is
    /// unconditional — the folder configs themselves go back unchanged, and
    /// `apply_folder_configs` re-reads the globals live.
    pub(in super::super) async fn apply_primary_project_signature_fallback(
        &self,
        primary_folder: Option<&Uri>,
        primary_project: &serde_json::Value,
    ) {
        let mut parsed = self.folder_configs.lock().await.clone();
        if let Some(primary_folder) = primary_folder {
            let project_override = signature_help_disabled_commands(primary_project);
            if let Some((_, config)) = parsed
                .iter_mut()
                .find(|(folder, _)| folder == primary_folder)
            {
                config.signature_help_disabled_commands = project_override;
            } else if let Some(commands) = project_override {
                parsed.push((
                    primary_folder.clone(),
                    FolderConfig {
                        signature_help_disabled_commands: Some(commands),
                        ..FolderConfig::default()
                    },
                ));
            }
        }
        self.apply_folder_configs(parsed).await;
    }

    /// The whole `didChangeConfiguration` reload pipeline, run **once** for a
    /// burst of notifications instead of once per notification.
    ///
    /// Uncoalesced, a settings-editor burst of 16 notifications produces 32
    /// `workspace/configuration` batches (one unscoped request plus one scoped
    /// batch per folder, each time) and re-analyses every open buffer 16 times.
    ///
    /// Coalescing is leader-and-dirty-flag rather than a plain trailing
    /// debounce, because the ordering guarantee matters: the settings the *last*
    /// notification carried must be pulled, applied, **and** re-resolved, so a
    /// superseded generation may be dropped only while a later one is
    /// guaranteed to run the whole pipeline. The first notification of a burst
    /// becomes the leader and every later one simply marks the state dirty and
    /// returns; the leader sleeps out the debounce window, then runs the
    /// pipeline for whatever the burst finally settled on, and loops if another
    /// notification arrived while it ran.
    ///
    /// Callers that must not be debounced (`initialized`, the `.tcl-lsp.ini`
    /// watcher, a workspace-folder change) call [`Self::pull_and_apply_config`]
    /// directly — they are one-shot events, not bursts, and start-up must not
    /// pay the window.
    pub(in super::super) async fn coalesced_config_reload(&self) -> DocumentSourceResult<()> {
        let lead = {
            let mut slot = self.config_reload.lock().await;
            slot.dirty = true;
            if slot.running {
                false
            } else {
                slot.running = true;
                true
            }
        };
        if !lead {
            return Ok(());
        }
        loop {
            crate::rt::sleep(CONFIG_RELOAD_DEBOUNCE).await;
            {
                let mut slot = self.config_reload.lock().await;
                if !slot.dirty {
                    slot.running = false;
                    return Ok(());
                }
                // Cleared *before* the run, so a notification that arrives
                // while it is in flight is seen as a new generation and gets
                // its own pass — never folded into a run that started before
                // its settings landed.
                slot.dirty = false;
            }
            self.run_config_reload().await?;
        }
    }

    /// One generation of the configuration reload: pull, apply, re-resolve
    /// dialects, re-run diagnostics, and ask the client to refresh the
    /// on-demand providers.
    pub(in super::super) async fn run_config_reload(&self) -> DocumentSourceResult<()> {
        self.pull_and_apply_config().await?;
        // `tclLsp.specPacks` is part of what was just applied: if it moved, the
        // pack set — and therefore the registry every open document resolves
        // against — moves with it.  The reload is content-keyed, so a settings
        // burst that does not touch `specPacks` costs one discovery walk and
        // nothing else.
        self.reload_spec_packs(ReloadTrigger::Config).await?;
        // The re-pull may have flipped `features.diagnostics`,
        // `optimiser.enabled`, or the disabled-diagnostics set — none of which
        // changes a document's dialect (the dialect the re-pull *does* resolve
        // is re-applied by `pull_and_apply_config`'s own re-resolve).  Re-run
        // diagnostics for every open buffer so the
        // new toggles take effect immediately (clearing squiggles when the
        // master switch goes off, dropping O-codes when the optimiser goes off)
        // rather than lingering until the next keystroke.
        self.reschedule_all_open_documents().await;
        // The same toggles govern closed files that still carry a badge:
        // a master-switch-off must clear their squiggles too, and a disabled-code
        // change must re-lint them — the open-document reschedule alone would
        // leave a closed file's badge frozen at its pre-toggle set.
        self.reschedule_closed_file_diagnostics().await;
        // On-demand providers cache their last result client-side and only
        // re-request on a document edit — a bare config change (e.g. toggling
        // `features.folding` off) would otherwise leave stale folding ranges /
        // code lenses on screen.  Ask the client to refresh them.  Best-effort:
        // a client without refresh support rejects the request, which is
        // harmless.  `foldingRange/refresh` (LSP 3.18) is not in ls-types, so it
        // is sent through a locally-defined request type.
        let _ = bounded_client_request(self.client.send_request::<FoldingRangeRefreshRequest>(()))
            .await;
        let _ = bounded_client_request(self.client.code_lens_refresh()).await;

        Ok(())
    }
}
