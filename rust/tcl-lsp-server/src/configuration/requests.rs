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

//! The `workspace/didChangeConfiguration` request handler.

use crate::{
    Backend, DidChangeConfigurationParams, MessageType, ReloadTrigger, config_ini,
    normalize_config_payload, settings_environment_kind_enabled, settings_non_ascii_mode,
    workspace_trust_in,
};

impl Backend {
    pub(crate) async fn serve_did_change_configuration(
        &self,
        params: DidChangeConfigurationParams,
    ) {
        // Accept ``{"tclLsp": {"dialect": "<name>"}}`` in either the
        // VS Code-style nested shape or the flat ``{"dialect":
        // "<name>"}`` shape (used by the MCP bridge). Update the
        // session default so newly opened documents pick up the change,
        // then re-resolve every already-open document under the new
        // default and reschedule diagnostics for any whose dialect
        // changed — `tclLsp.selectDialect` pushes a dialect-only config
        // change for a buffer that is already open, and without this the
        // buffer would keep being parsed/diagnosed under the dialect it
        // was opened with until reopened. This snapshots each open
        // document's resolved dialect and force-reanalyses the ones that
        // change.
        let dialect = params
            .settings
            .get("tclLsp")
            .and_then(|v| v.get("dialect"))
            .or_else(|| params.settings.get("dialect"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        // The editor's Workspace Trust state, when the client pushes it — VS
        // Code does when the user grants trust. Top-level only: a `tclLsp`
        // section here is the synchronised settings, which the workspace's
        // own settings file can write (`workspace_trust_in`).
        let analyser_inputs_guard = self.analyser_inputs_gate.write().await;
        let trust_changed = match workspace_trust_in(&params.settings) {
            Some(trust) => {
                let mut current = self.workspace_trust.lock().await;
                let changed = *current != trust;
                *current = trust;
                changed
            }
            None => false,
        };
        let mut dialect_warning = None;
        if let Some(d) = dialect {
            dialect_warning = self.apply_configured_session_dialect(&d).await;
            // No re-resolve here: the coalesced reload below ends with one
            // (`pull_and_apply_config`'s own, at the single point every pull
            // passes through), so doing it per notification would only
            // multiply the re-analyses a settings-editor burst causes.
        }
        // W108 mode + disabled-diagnostics reconfiguration. Existing
        // documents pick the change up on their next analyse (the
        // analyses cache is keyed per URI and refreshed on edit).
        if let Some(mode) = settings_non_ascii_mode(&params.settings) {
            *self.non_ascii_mode.lock().await = mode;
        }
        let skip = {
            let mut layers = self.policy_layers.lock().await;
            layers.editor = config_ini::merge_settings(
                &layers.editor,
                &normalize_config_payload(&params.settings),
            );
            layers.production_skip()
        };
        *self.disabled_diagnostics.lock().await = skip;
        if let Some(flag) = settings_environment_kind_enabled(&params.settings) {
            self.environment_notice.set_enabled(flag);
        }
        // The writes above land immediately, ahead of the coalesced
        // re-pull below, so they retire the scheduler's cached inputs on their
        // own — the flat MCP-bridge payload is the only thing that carries them
        // and it must not need a second notification to take effect on the next
        // keystroke.
        self.invalidate_diag_inputs();
        drop(analyser_inputs_guard);
        // A trust change starts or stops a workspace pack's hook bodies. The
        // pack-set key mixes the state, so this reload re-installs with no
        // file moved, and every open document re-analyses against the bodies
        // that now run — or no longer do.
        if trust_changed
            && (match self.reload_spec_packs(ReloadTrigger::Trust).await {
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
        self.warn_configured_dialect(dialect_warning).await;
        // VS Code (and the e2e harness) push an empty/partial payload as a
        // signal to re-pull the full resolved config via
        // `workspace/configuration`.  Always re-pull so `features.*`, the
        // optimiser switch, and the analyser knobs reflect the latest editor
        // settings — the inline `params.settings` handling above covers the
        // flat MCP-bridge shape that carries the values directly.  Coalesced:
        // a settings-editor burst is one pull and one re-analysis, not one per
        // notification.
        match self.coalesced_config_reload().await {
            Ok(value) => value,
            Err(error) => {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
            }
        }
    }
}
