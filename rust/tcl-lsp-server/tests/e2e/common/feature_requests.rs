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

//! One method per LSP feature request, with the settled forms of the semantic-token requests.

use super::*;

impl Lsp {
    // Feature requests.

    fn doc_pos(uri: &str, line: u32, ch: u32) -> Value {
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": ch },
        })
    }

    pub fn hover(&mut self, uri: &str, line: u32, ch: u32) -> Value {
        self.request("textDocument/hover", Self::doc_pos(uri, line, ch))
    }

    pub fn completion(&mut self, uri: &str, line: u32, ch: u32) -> Value {
        self.request("textDocument/completion", Self::doc_pos(uri, line, ch))
    }

    pub fn signature_help(&mut self, uri: &str, line: u32, ch: u32) -> Value {
        self.request("textDocument/signatureHelp", Self::doc_pos(uri, line, ch))
    }

    pub fn definition(&mut self, uri: &str, line: u32, ch: u32) -> Value {
        self.request("textDocument/definition", Self::doc_pos(uri, line, ch))
    }

    pub fn type_definition(&mut self, uri: &str, line: u32, ch: u32) -> Value {
        self.request("textDocument/typeDefinition", Self::doc_pos(uri, line, ch))
    }

    pub fn declaration(&mut self, uri: &str, line: u32, ch: u32) -> Value {
        self.request("textDocument/declaration", Self::doc_pos(uri, line, ch))
    }

    pub fn references(&mut self, uri: &str, line: u32, ch: u32, include_decl: bool) -> Value {
        let mut params = Self::doc_pos(uri, line, ch);
        params["context"] = json!({ "includeDeclaration": include_decl });
        self.request("textDocument/references", params)
    }

    pub fn document_highlight(&mut self, uri: &str, line: u32, ch: u32) -> Value {
        self.request(
            "textDocument/documentHighlight",
            Self::doc_pos(uri, line, ch),
        )
    }

    pub fn document_symbols(&mut self, uri: &str) -> Value {
        self.request(
            "textDocument/documentSymbol",
            json!({ "textDocument": { "uri": uri } }),
        )
    }

    pub fn workspace_symbols(&mut self, query: &str) -> Value {
        self.request("workspace/symbol", json!({ "query": query }))
    }

    pub fn semantic_tokens(&mut self, uri: &str) -> Value {
        self.request(
            "textDocument/semanticTokens/full",
            json!({ "textDocument": { "uri": uri } }),
        )
    }

    /// `semanticTokens/full` for `uri`, converged onto the **settled enriched**
    /// stream the way a conformant editor does.
    ///
    /// A bare `semantic_tokens` call races
    /// `SEMANTIC_TOKENS_FAST_PATH_BUDGET`: when the enriched (SSA/SCCP-informed)
    /// computation overruns 40 ms the server answers from the cheap coarse tier
    /// and pushes `workspace/semanticTokens/refresh` once the enriched stream
    /// lands. That is correct server behaviour and an editor re-requests — but a
    /// *test* that asserts on enrichment (a regex source retagged, a user-class
    /// method resolved) and reads only the first response is asserting on
    /// whichever tier happened to win, i.e. on how much CPU the machine had.
    /// Those tests pass on a quiet box and fail under parallel load, which is
    /// not a server defect.
    ///
    /// So this converges the way the client contract says to, driven by the
    /// server's own settled marker rather than by sleeps: request, wait for the
    /// convergence decision to be logged, and re-request when it says a
    /// refresh was warranted. A cancelled read or a repeat-suppressed refresh
    /// also re-races: neither has served the enriched stream. Only a response
    /// marked `served-enriched`, `compared-equal`, or `no-analysis` is final.
    /// Missing convergence evidence or a required refresh fails within the
    /// same deadline; genuine token-content divergence remains the caller's
    /// assertion to report.
    pub fn semantic_tokens_settled(&mut self, uri: &str) -> Value {
        let deadline = Instant::now() + scaled_timeout(DEFAULT_TIMEOUT);
        loop {
            assert!(
                Instant::now() < deadline,
                "semantic token full result did not settle within {DEFAULT_TIMEOUT:?} for {uri}{}",
                latency_barrier_timeout_note()
            );
            let req_since = self.server_request_cursor();
            let log_since = self.notification_cursor();
            let response = self.request_before(
                "textDocument/semanticTokens/full",
                json!({ "textDocument": { "uri": uri } }),
                DEFAULT_TIMEOUT,
                deadline,
            );
            let Some(settled) = self.try_await_log_of_type_before(
                &["semantic_tokens.full_convergence.settled", uri],
                None,
                deadline,
                log_since,
            ) else {
                panic!(
                    "semantic token full convergence marker missing before {DEFAULT_TIMEOUT:?} deadline for {uri}{}",
                    latency_barrier_timeout_note()
                );
            };
            assert!(
                Instant::now() < deadline,
                "semantic token full convergence marker arrived after {DEFAULT_TIMEOUT:?} deadline for {uri}{}",
                latency_barrier_timeout_note()
            );
            if settled.contains("refresh=true") || settled.contains("outcome=coalesced") {
                if self
                    .try_await_server_request_before(
                        "workspace/semanticTokens/refresh",
                        deadline,
                        req_since,
                    )
                    .is_none()
                {
                    panic!(
                        "semantic token full refresh was required but absent before {DEFAULT_TIMEOUT:?} deadline for {uri}{}",
                        latency_barrier_timeout_note()
                    );
                }
                assert!(
                    Instant::now() < deadline,
                    "semantic token full refresh arrived after {DEFAULT_TIMEOUT:?} deadline for {uri}{}",
                    latency_barrier_timeout_note()
                );
            } else if settled.contains("outcome=served-enriched")
                || settled.contains("outcome=compared-equal")
                || settled.contains("outcome=no-analysis")
            {
                assert!(
                    Instant::now() < deadline,
                    "semantic token full result settled after {DEFAULT_TIMEOUT:?} deadline for {uri}{}",
                    latency_barrier_timeout_note()
                );
                return response;
            }
            // A cancelled read, a repeat-suppressed refresh, or an unexpected
            // marker has not established that this response is enriched. Retry
            // only while the original request/marker/refresh deadline remains.
        }
    }

    pub fn semantic_tokens_range(
        &mut self,
        uri: &str,
        start: (u32, u32),
        end: (u32, u32),
    ) -> Value {
        self.request(
            "textDocument/semanticTokens/range",
            json!({
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": start.0, "character": start.1 },
                    "end": { "line": end.0, "character": end.1 },
                },
            }),
        )
    }

    /// Request viewport tokens until the server proves the returned stream is
    /// the enriched one (or that the coarse stream compared equal). This is
    /// the range counterpart of [`Self::semantic_tokens_settled`].
    pub fn semantic_tokens_range_settled(
        &mut self,
        uri: &str,
        start: (u32, u32),
        end: (u32, u32),
    ) -> Value {
        let deadline = Instant::now() + scaled_timeout(DEFAULT_TIMEOUT);
        loop {
            assert!(
                Instant::now() < deadline,
                "semantic token range result did not settle within {DEFAULT_TIMEOUT:?} for {uri}{}",
                latency_barrier_timeout_note()
            );
            let req_since = self.server_request_cursor();
            let log_since = self.notification_cursor();
            let params = json!({
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": start.0, "character": start.1 },
                    "end": { "line": end.0, "character": end.1 },
                },
            });
            let response = self.request_before(
                "textDocument/semanticTokens/range",
                params,
                DEFAULT_TIMEOUT,
                deadline,
            );
            let Some(settled) = self.try_await_log_of_type_before(
                &["semantic_tokens.range_convergence.settled", uri],
                None,
                deadline,
                log_since,
            ) else {
                panic!(
                    "semantic token range convergence marker missing before {DEFAULT_TIMEOUT:?} deadline for {uri}{}",
                    latency_barrier_timeout_note()
                );
            };
            assert!(
                Instant::now() < deadline,
                "semantic token range convergence marker arrived after {DEFAULT_TIMEOUT:?} deadline for {uri}{}",
                latency_barrier_timeout_note()
            );
            if settled.contains("refresh=true") || settled.contains("outcome=coalesced") {
                if self
                    .try_await_server_request_before(
                        "workspace/semanticTokens/refresh",
                        deadline,
                        req_since,
                    )
                    .is_none()
                {
                    panic!(
                        "semantic token range refresh was required but absent before {DEFAULT_TIMEOUT:?} deadline for {uri}{}",
                        latency_barrier_timeout_note()
                    );
                }
                assert!(
                    Instant::now() < deadline,
                    "semantic token range refresh arrived after {DEFAULT_TIMEOUT:?} deadline for {uri}{}",
                    latency_barrier_timeout_note()
                );
            } else if settled.contains("outcome=served-enriched")
                || settled.contains("outcome=compared-equal")
            {
                assert!(
                    Instant::now() < deadline,
                    "semantic token range result settled after {DEFAULT_TIMEOUT:?} deadline for {uri}{}",
                    latency_barrier_timeout_note()
                );
                return response;
            }
            // A suppressed refresh, cancelled read, or coalesced continuation
            // establishes no enriched answer for this response. Retry only
            // within the same original deadline.
        }
    }

    pub fn implementation(&mut self, uri: &str, line: u32, ch: u32) -> Value {
        self.request("textDocument/implementation", Self::doc_pos(uri, line, ch))
    }

    pub fn document_links(&mut self, uri: &str) -> Value {
        self.request(
            "textDocument/documentLink",
            json!({ "textDocument": { "uri": uri } }),
        )
    }

    pub fn code_lens(&mut self, uri: &str) -> Value {
        self.request(
            "textDocument/codeLens",
            json!({ "textDocument": { "uri": uri } }),
        )
    }

    pub fn code_lens_resolve(&mut self, lens: Value) -> Value {
        self.request("codeLens/resolve", lens)
    }

    pub fn inlay_hints(&mut self, uri: &str, start: (u32, u32), end: (u32, u32)) -> Value {
        self.request(
            "textDocument/inlayHint",
            json!({
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": start.0, "character": start.1 },
                    "end": { "line": end.0, "character": end.1 },
                },
            }),
        )
    }

    pub fn linked_editing_range(&mut self, uri: &str, line: u32, ch: u32) -> Value {
        self.request(
            "textDocument/linkedEditingRange",
            Self::doc_pos(uri, line, ch),
        )
    }

    pub fn prepare_call_hierarchy(&mut self, uri: &str, line: u32, ch: u32) -> Value {
        self.request(
            "textDocument/prepareCallHierarchy",
            Self::doc_pos(uri, line, ch),
        )
    }

    pub fn incoming_calls(&mut self, item: Value) -> Value {
        let mut params = json!({});
        params["item"] = item;
        self.request("callHierarchy/incomingCalls", params)
    }

    pub fn outgoing_calls(&mut self, item: Value) -> Value {
        let mut params = json!({});
        params["item"] = item;
        self.request("callHierarchy/outgoingCalls", params)
    }

    pub fn prepare_rename(&mut self, uri: &str, line: u32, ch: u32) -> Value {
        self.request("textDocument/prepareRename", Self::doc_pos(uri, line, ch))
    }

    pub fn rename(&mut self, uri: &str, line: u32, ch: u32, new_name: &str) -> Value {
        let mut params = Self::doc_pos(uri, line, ch);
        params["newName"] = json!(new_name);
        self.request("textDocument/rename", params)
    }

    /// The rename request's JSON-RPC `error` object, or `null` when the
    /// request succeeded — the wire view of a safety-gate refusal.
    pub fn rename_error(&mut self, uri: &str, line: u32, ch: u32, new_name: &str) -> Value {
        let mut params = Self::doc_pos(uri, line, ch);
        params["newName"] = json!(new_name);
        self.request_response("textDocument/rename", params, REQUEST_TIMEOUT)
            .get("error")
            .cloned()
            .unwrap_or(Value::Null)
    }

    pub fn folding_range(&mut self, uri: &str) -> Value {
        self.request(
            "textDocument/foldingRange",
            json!({ "textDocument": { "uri": uri } }),
        )
    }

    pub fn selection_range(&mut self, uri: &str, positions: Value) -> Value {
        let mut params = json!({ "textDocument": { "uri": uri } });
        params["positions"] = positions;
        self.request("textDocument/selectionRange", params)
    }

    pub fn formatting(&mut self, uri: &str, tab_size: u32, insert_spaces: bool) -> Value {
        self.request(
            "textDocument/formatting",
            json!({
                "textDocument": { "uri": uri },
                "options": { "tabSize": tab_size, "insertSpaces": insert_spaces },
            }),
        )
    }

    pub fn range_formatting(&mut self, uri: &str, range: Value, tab_size: u32) -> Value {
        let mut params = json!({
            "textDocument": { "uri": uri },
            "options": { "tabSize": tab_size, "insertSpaces": true },
        });
        params["range"] = range;
        self.request("textDocument/rangeFormatting", params)
    }

    pub fn code_actions(&mut self, uri: &str, range: Value, diagnostics: Value) -> Value {
        let mut params = json!({ "textDocument": { "uri": uri }, "context": {} });
        params["range"] = range;
        params["context"]["diagnostics"] = diagnostics;
        self.request("textDocument/codeAction", params)
    }

    pub fn execute_command(&mut self, command: &str, arguments: Value) -> Value {
        let mut params = json!({ "command": command });
        params["arguments"] = arguments;
        self.request("workspace/executeCommand", params)
    }
}
