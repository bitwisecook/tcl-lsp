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

//! Starting a server for a test: the per-dialect constructors, configured and rooted variants, and the `initialize` handshake.

use super::*;

impl Lsp {
    /// Spawn + initialise a server whose `workspace/configuration` reply enables
    /// only linked editing — the default-editor contract every plain Tcl test
    /// runs against.
    pub fn tcl() -> Self {
        Self::with_config(json!({ "features": { "linkedEditingRange": true } }))
    }

    /// [`Lsp::tcl`] analysing as Tcl 9.0, for surfaces 8.6 does not have
    /// (`classmethod` is `invalid command name` under tclsh 8.6).
    pub fn tcl9() -> Self {
        Self::with_config(json!({
            "dialect": "tcl9.0",
            "features": { "linkedEditingRange": true },
        }))
    }

    /// [`Lsp::tcl`] with test-only environment seams installed on the child.
    pub fn tcl_with_env(env: &[(&str, &str)]) -> Self {
        Self::with_config_env(json!({ "features": { "linkedEditingRange": true } }), env)
    }

    /// A server dedicated to iRules-dialect documents (dialect switch is
    /// process-global, so those tests use their own server). Same config as
    /// [`Lsp::tcl`].
    pub fn irules() -> Self {
        Self::tcl()
    }

    /// A server with inlay hints opted in (default-off otherwise).
    pub fn inlay() -> Self {
        Self::with_config(json!({
            "features": {
                "linkedEditingRange": true,
                "inlayTypeHints": true,
                "inlayParameterHints": true,
            }
        }))
    }

    /// A server for BIG-IP config documents. Same config as [`Lsp::tcl`].
    pub fn bigip() -> Self {
        Self::tcl()
    }

    /// Spawn + initialise a server whose `tclLsp` configuration reply is
    /// `config`.
    /// Start a server whose `workspace/configuration` reply is `config`, and
    /// **block until that config has actually been applied**.
    ///
    /// The barrier is load-bearing, not belt-and-braces.  The server pulls the
    /// config from inside its `initialized` handler, which runs *concurrently*
    /// with any `didOpen` the client has already queued (see the comment on
    /// `Backend::initialized`).  Without this wait, a document opened right
    /// after construction races the pull and is routinely analysed under the
    /// *default* config — e.g. a test that opts into `optimiser.profile =
    /// standard` and then asserts on an O102 diagnostic gets an empty
    /// diagnostic set, because O102 is outside the default `readability`
    /// profile.  The server does converge (its `initialized` tail reschedules
    /// every open document), so the corrected publish arrives later — but a
    /// test that sampled the first publish has already failed by then.  That
    /// was a real, reproducible flake in `vscode_parity`'s two optimisation
    /// -payload tests.
    ///
    /// Settling here rather than in each test fixes the whole class: every
    /// `with_config` caller is now guaranteed that its config is in effect
    /// before it opens anything.
    pub fn with_config(config: Value) -> Self {
        Self::with_config_env(config, &[])
    }

    /// [`Lsp::with_config`] with test-only environment seams installed on the
    /// child process.
    pub fn with_config_env(config: Value, env: &[(&str, &str)]) -> Self {
        let mut lsp = Self::spawn_with_env(config, env);
        lsp.initialize();
        // Settle on exactly what this client will reply to
        // `workspace/configuration` with, read back from the shared slot.
        let requested = lsp.shared.tcllsp_config.lock().unwrap().clone();
        lsp.settle_config(&requested);
        // The startup pack reload and workspace scan run concurrently with the
        // first `didOpen`. Each replaces the analyser inputs, so a document
        // opened before they land is analysed twice, and a request sent between
        // the reload and the second analysis is answered as unavailable (an
        // empty result). A test that means to race the startup reload holds it
        // through the environment seam and drives the interleaving itself.
        if !env.iter().any(|(name, _)| *name == STARTUP_RELOAD_HOLD_ENV) {
            lsp.await_notification("tcl-lsp/specPacksReloaded", DEFAULT_TIMEOUT);
            lsp.await_log(&["workspace_folders_scan"], DEFAULT_TIMEOUT, 0);
        }
        lsp
    }

    /// The isolated XDG root this server runs under: `config/`, `cache/` and
    /// `state/` beneath it are the server's `XDG_*_HOME` unless a test named
    /// its own.
    pub fn xdg_root(&self) -> &std::path::Path {
        &self.xdg_root
    }

    /// Set the client capabilities the next `initialize` advertises. The
    /// default is none, the least a client can offer.
    pub fn set_client_capabilities(&mut self, capabilities: Value) {
        self.client_capabilities = capabilities;
    }

    /// Choose the action this client picks when the server sends
    /// `window/showMessageRequest`; `None` dismisses the message unanswered.
    pub fn choose_message_action(&self, title: Option<&str>) {
        *self.shared.message_action_reply.lock().unwrap() = title.map(str::to_owned);
    }

    /// Run the `initialize` handshake and send `initialized`.
    pub fn initialize(&mut self) -> Value {
        self.initialize_at(harness_root_uri())
    }

    /// [`Lsp::initialize`] against an explicit workspace-root URI.
    pub fn initialize_at(&mut self, root: &str) -> Value {
        let result = self.request_timeout(
            "initialize",
            json!({
                "processId": std::process::id(),
                "rootUri": root,
                "workspaceFolders": [{ "uri": root, "name": "e2e" }],
                "capabilities": self.client_capabilities.clone(),
                "clientInfo": { "name": "tcl-lsp-e2e", "version": "1.0" },
            }),
            REQUEST_TIMEOUT,
        );
        self.initialize_result = result.clone();
        self.notify("initialized", json!({}));
        result
    }

    /// Spawn a server rooted at a **real on-disk directory** and return the
    /// moment the `initialize` handshake is done — no config settle, no wait
    /// for the startup workspace scan.
    ///
    /// [`Lsp::tcl`] deliberately settles the config first, which incidentally
    /// gives the startup scan time to finish; a test that means to *race* that
    /// scan (the way a real editor's first `workspace/symbol` does) must not
    /// pay that barrier.
    pub fn at_workspace_root(root: &std::path::Path) -> Self {
        let mut lsp = Self::spawn(json!({ "features": { "linkedEditingRange": true } }));
        let root_uri = tcl_lsp_server::canonical_file_uri(root)
            .expect("workspace root has a representable file URI")
            .to_string();
        lsp.initialize_at(&root_uri);
        lsp
    }

    /// [`Lsp::with_config`] rooted at a **real on-disk directory**.
    ///
    /// The combination the two existing constructors do not cover, and the one
    /// any workspace-scope feature needs: a config the server will actually
    /// apply *and* a root it can walk. Settles the config first, for the same
    /// reason [`Lsp::with_config`] does — a test that opens a document before
    /// its settings have landed is testing the defaults.
    pub fn with_config_at_root(config: Value, root: &std::path::Path) -> Self {
        Self::with_config_at_root_env(config, root, &[])
    }

    /// [`Lsp::with_config_at_root`] with extra environment variables on the
    /// server process — see [`Lsp::spawn_with_env`].
    ///
    /// Settling the config here does **not** settle the startup *pack* reload:
    /// the server pulls and applies its configuration before it starts
    /// loading packs, so a test that means to race that load still can.
    pub fn with_config_at_root_env(
        config: Value,
        root: &std::path::Path,
        env: &[(&str, &str)],
    ) -> Self {
        let mut lsp = Self::spawn_with_env(config, env);
        let root_uri = tcl_lsp_server::canonical_file_uri(root)
            .expect("workspace root has a representable file URI")
            .to_string();
        lsp.initialize_at(&root_uri);
        let requested = lsp.shared.tcllsp_config.lock().unwrap().clone();
        lsp.settle_config(&requested);
        lsp
    }

    /// Spawn a **multi-root** server: `unscoped` is what the workspace-wide
    /// `workspace/configuration` pull returns, and each `(root, folder_config)`
    /// pair is what the *scoped* pull for that folder returns.
    ///
    /// This is the shape a real multi-root editor answers with — folder-level
    /// settings are invisible to the unscoped pull — so it is the only way to
    /// exercise per-folder resolution end to end.
    pub fn multi_root(unscoped: Value, folders: &[(&std::path::Path, Value)]) -> Self {
        let mut lsp = Self::spawn(unscoped);
        {
            let mut map = lsp.shared.folder_configs.lock().unwrap();
            for (root, config) in folders {
                let uri = tcl_lsp_server::canonical_file_uri(root)
                    .expect("workspace folder has a representable file URI")
                    .to_string();
                map.insert(uri, config.clone());
            }
        }
        let roots: Vec<Value> = folders
            .iter()
            .enumerate()
            .map(|(i, (root, _))| {
                let uri = tcl_lsp_server::canonical_file_uri(root)
                    .expect("workspace folder has a representable file URI")
                    .to_string();
                json!({ "uri": uri, "name": format!("root{i}") })
            })
            .collect();
        lsp.initialize_with_folders(&roots);
        lsp
    }

    /// [`Lsp::initialize`] against an explicit `workspaceFolders` array.
    pub fn initialize_with_folders(&mut self, folders: &[Value]) -> Value {
        let root = folders
            .first()
            .and_then(|f| f.get("uri"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let result = self.request_timeout(
            "initialize",
            json!({
                "processId": std::process::id(),
                "rootUri": root,
                "workspaceFolders": folders,
                "capabilities": self.client_capabilities.clone(),
                "clientInfo": { "name": "tcl-lsp-e2e", "version": "1.0" },
            }),
            REQUEST_TIMEOUT,
        );
        self.initialize_result = result.clone();
        self.notify("initialized", json!({}));
        result
    }

    /// The full `initialize` result.
    pub fn initialize_result(&self) -> &Value {
        &self.initialize_result
    }

    /// `serverInfo` from the `initialize` result, if present.
    pub fn server_info(&self) -> Option<&Value> {
        self.initialize_result.get("serverInfo")
    }
}
