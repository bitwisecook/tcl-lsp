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

//! The `initialize`, `initialized` and `shutdown` request handlers.

use crate::{
    Backend, FoldingRangeRefreshRequest, InitializeParams, InitializeResult, InitializedParams,
    MessageType, ReloadTrigger, ServerInfo, WorkspaceStartupGuard, bounded_client_request,
    build_server_capabilities, client_lacks_utf16_support, client_supports_conditional_rename,
    client_supports_message_action_items, client_supports_pull_diagnostics,
    client_supports_relative_watch_patterns, client_supports_show_document, jsonrpc,
    negotiate_position_encoding,
};

impl Backend {
    pub(crate) async fn serve_initialize(
        &self,
        params: InitializeParams,
    ) -> jsonrpc::Result<InitializeResult> {
        self.apply_workspace_folders(&params).await;
        self.apply_initialization_options(&params).await;
        // Push is the sole diagnostics channel by default: pull is opt-in and
        // the server does not advertise `diagnosticProvider` (see
        // `build_server_capabilities`).  A client that *supports* pull will not
        // actually pull unless the server advertises it, so the
        // "stop pushing when the client pulls" suppression must stay OFF here —
        // otherwise a pull-capable client (VS Code advertises the capability)
        // gets neither push (suppressed) nor pull (unadvertised), i.e. zero
        // diagnostics.  When pull is opted back in this flag is set from the
        // client's actual `textDocument/diagnostic` support via
        // `client_supports_pull_diagnostics(&params)`.
        let _ = client_supports_pull_diagnostics(&params);
        self.client_supports_pull_diagnostics
            .store(false, std::sync::atomic::Ordering::Relaxed);
        self.client_supports_relative_watch_patterns.store(
            client_supports_relative_watch_patterns(&params),
            std::sync::atomic::Ordering::Relaxed,
        );
        self.conditional_variable_rename_supported.store(
            client_supports_conditional_rename(&params),
            std::sync::atomic::Ordering::Relaxed,
        );
        self.environment_notice.set_client_capabilities(
            client_supports_message_action_items(&params),
            client_supports_show_document(&params),
        );
        let position_encoding = negotiate_position_encoding(&params);
        if client_lacks_utf16_support(&params) {
            // The client advertised position encodings without UTF-16. The
            // server emits UTF-16 columns only, so it proceeds in UTF-16
            // regardless (rather than failing the session over a baseline
            // every conformant client supports) — but warn so a genuine
            // UTF-8/UTF-32-only client's mis-mapped non-ASCII ranges are
            // diagnosable instead of silent.
            self.client
                .log_message(
                    MessageType::WARNING,
                    "client advertised position encodings without UTF-16; this server \
                     emits UTF-16 columns and will use UTF-16 regardless",
                )
                .await;
        }
        Ok(InitializeResult {
            capabilities: build_server_capabilities(position_encoding),
            server_info: Some(ServerInfo {
                // Protocol identity must match the editor's
                // expectations ("tcl-lsp"), not the crate/binary name.
                name: "tcl-lsp".to_owned(),
                // The release version from the tag, not the manifest's 0.1.0.
                version: Some(tcl_version::VERSION.to_owned()),
            }),
            ..Default::default()
        })
    }

    pub(crate) async fn serve_initialized(&self, _: InitializedParams) {
        let _startup_guard = WorkspaceStartupGuard(&self.workspace_scan_ready);
        self.client
            .log_message(MessageType::INFO, "tcl-lsp-server initialised")
            .await;
        if let Some(warning) = self.environment_notice.take_load_warning() {
            self.client.log_message(MessageType::WARNING, warning).await;
        }
        // Pull the resolved `tclLsp` config once the client is ready so
        // feature toggles / optimiser switch / analyser knobs are in effect
        // before the first request.
        match self.pull_and_apply_config().await {
            Ok(value) => value,
            Err(error) => {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        }
        // Ask the client to watch Tcl source files so external edits
        // (`git checkout`, generated files, deletions) reach
        // `did_change_watched_files` and refresh the cross-document index.
        // Best-effort: clients without dynamic-registration support reject
        // this, which is harmless — the startup scan still seeds the index.
        self.register_file_watchers().await;
        // Load the workspace's SpecTcl packs before the first analysis so a
        // pack-declared command is known from the first keystroke rather than
        // flashing W123 until a reload catches up.  `pull_and_apply_config`
        // above has already applied `tclLsp.specPacks`, so discovery sees the
        // configured paths on this first pass.
        match self.reload_spec_packs(ReloadTrigger::Startup).await {
            Ok(value) => value,
            Err(error) => {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        };
        // Type-hierarchy support is advertised statically by the
        // `inject_type_hierarchy_provider` service layer (see `main.rs`) — a
        // dynamic `client/registerCapability` here would both duplicate it and
        // fail to surface in the client's `initializeResult.capabilities`.
        // Seed the cross-document index with on-disk project files
        // the editor hasn't opened yet.
        match self.scan_workspace_folders().await {
            Ok(value) => value,
            Err(error) => {
                self.client
                    .log_message(MessageType::WARNING, error.to_string())
                    .await;
                return;
            }
        }
        // `initialized` fires concurrently with any `textDocument/didOpen` the
        // client already queued for its restored tabs (`buffer_unordered`, see
        // `edit_serialize`'s doc comment) — a document's first debounced
        // diagnostics run (50ms) routinely completes before this scan does on
        // anything but a trivial workspace. The always-on W120/W123 workspace
        // refinement (`refine_workspace_w120`/`refine_workspace_w123`) reads
        // `workspace_index` / `package_resolver`, which this scan is what
        // populates — so a document opened at startup can publish a diagnostic
        // (e.g. a false W120 for a command whose `package require` lives in an
        // unopened `source` ancestor) computed against the still-empty stores,
        // and nothing republishes it once the scan finishes. Reschedule every
        // open document now so any diagnostics published before the scan
        // completed are refreshed against the now-populated workspace state —
        // the same pattern `did_change_watched_files` already uses after its
        // own `scan_workspace_folders` call on a config change. The populated
        // package database is a diagnostics input, so a report computed
        // against the empty one is stale rather than complete.
        self.invalidate_diag_inputs();
        self.reschedule_all_open_documents().await;
        // VS Code computes a document's sticky-scroll model once, when the
        // editor opens, and a folding provider that registers *after* that
        // first computation does not itself trigger a recompute — the tabs
        // restored at startup would keep an empty sticky bar until the next
        // edit.  `workspace/foldingRange/refresh` does trigger one (it fires
        // `SyntaxRangeProvider.onDidChange`), so send it once the provider is
        // genuinely live.  Best-effort and idempotent, exactly as in
        // `did_change_configuration`: a client without refresh support
        // rejects the request, which is harmless.
        let _ = bounded_client_request(self.client.send_request::<FoldingRangeRefreshRequest>(()))
            .await;
    }

    pub(crate) async fn serve_shutdown(&self) -> jsonrpc::Result<()> {
        self.workspace_scan_ready.shutdown.raise();
        let retired = self.studio_sources.lock().await.close();
        if retired {
            self.reload_spec_packs(ReloadTrigger::StudioSource).await?;
            if !self
                .studio_sources
                .lock()
                .await
                .shutdown_publication_complete()
            {
                return Err(jsonrpc::Error {
                    code: jsonrpc::ErrorCode::InternalError,
                    message: "Studio source withdrawal publication is unavailable".into(),
                    data: None,
                });
            }
        }
        Ok(())
    }
}
