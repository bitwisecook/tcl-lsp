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

//! Document lifecycle notifications, and opening a document up to its ready analysis snapshot.

use super::*;

impl Lsp {
    // Document lifecycle.

    pub fn open_document(&mut self, uri: &str, text: &str) {
        self.open_document_lang(uri, text, "tcl", 1);
    }

    pub fn open_document_lang(&mut self, uri: &str, text: &str, language_id: &str, version: i64) {
        self.notify(
            "textDocument/didOpen",
            json!({ "textDocument": {
                "uri": uri, "languageId": language_id, "version": version, "text": text,
            }}),
        );
        self.open_uris.push(uri.to_owned());
    }

    /// Send a `didChange` with the raw `contentChanges` array.
    pub fn change_document(&mut self, uri: &str, version: i64, changes: Value) {
        let mut params = json!({ "textDocument": { "uri": uri, "version": version } });
        params["contentChanges"] = changes;
        self.notify("textDocument/didChange", params);
    }

    /// A full-text replace at `version`.
    pub fn replace_document(&mut self, uri: &str, version: i64, text: &str) {
        self.change_document(uri, version, json!([{ "text": text }]));
    }

    pub fn close_document(&mut self, uri: &str) {
        self.notify(
            "textDocument/didClose",
            json!({ "textDocument": { "uri": uri } }),
        );
        self.open_uris.retain(|u| u != uri);
    }

    /// Open `text` and block until its analysis snapshot is ready, returning the
    /// published diagnostics. Waits on both the version-tagged diagnostics and
    /// the per-URI `workspace_state.update` log line.
    pub fn open_ready(&mut self, uri: &str, text: &str) -> Vec<Value> {
        self.open_ready_lang(uri, text, "tcl")
    }

    /// Like [`Lsp::open_ready`] but with an explicit `languageId` (e.g.
    /// `"tcl-irule"` for iRules, `"tk"` for Tk, `"bigip"` for BIG-IP config).
    pub fn open_ready_lang(&mut self, uri: &str, text: &str, language_id: &str) -> Vec<Value> {
        self.open_ready_lang_timeout(uri, text, language_id, DEFAULT_TIMEOUT)
    }

    /// [`Lsp::open_ready`] with an explicit barrier backstop.
    ///
    /// [`DEFAULT_TIMEOUT`] sizes the barrier for an ordinary test document —
    /// tens of lines, analysed in milliseconds — where 30 s is unmistakably a
    /// hang guard. A handful of tests open *thousands* of lines, and a full
    /// debug-build analysis of one of those, on a machine already running the
    /// rest of the suite in parallel, is legitimately tens of seconds: there the
    /// default stops being a hang guard and becomes a bet on how many cores the
    /// test happened to get. Those call sites pass their own backstop rather
    /// than the whole harness inheriting a number sized for the worst case.
    /// (Load scaling still applies on top — see the module docs.)
    pub fn open_ready_timeout(&mut self, uri: &str, text: &str, timeout: Duration) -> Vec<Value> {
        self.open_ready_lang_timeout(uri, text, "tcl", timeout)
    }

    /// [`Lsp::open_ready_lang`] with an explicit barrier backstop.
    pub fn open_ready_lang_timeout(
        &mut self,
        uri: &str,
        text: &str,
        language_id: &str,
        timeout: Duration,
    ) -> Vec<Value> {
        self.open_document_lang(uri, text, language_id, 1);
        let diags = self.await_diagnostics_version(uri, Some(1), timeout);
        self.await_log(&["workspace_state.update", uri], timeout, 0);
        diags
    }

    /// Force `uri`'s analysis snapshot to `text` at `version` and block until it
    /// is built. Returns `version`.
    ///
    /// The barrier is a **synchronous `textDocument/diagnostic` pull request**,
    /// not an await on a version-tagged `publishDiagnostics` push. That push is
    /// produced by the server's debounced, coalescing diagnostics scheduler, and
    /// a no-op full replace (`text` == the buffer's current content) is a salsa
    /// cache hit: the push tagged with this *exact* version can be legitimately
    /// coalesced away or delayed far past the timeout under CI load, so awaiting
    /// it tripped the barrier's 30s timeout intermittently — a flake, not an
    /// oracle divergence. The pull handler instead reads the *live* buffer and
    /// returns once diagnostics for the document's current revision are settled
    /// (served from the push cache when it already published that revision, else
    /// computed synchronously on demand). It is therefore a deterministic
    /// "analysis settled at the final content" signal the debounce/coalescer
    /// cannot suppress, with [`Lsp::request`]'s own timeout as the safety net.
    pub fn settle_analysis(&mut self, uri: &str, version: i64, text: &str) -> i64 {
        self.replace_document(uri, version, text);
        let _ = self.pull_diagnostics(uri);
        version
    }
}
