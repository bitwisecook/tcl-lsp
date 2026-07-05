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

//! Native LSP server binary entry point.
//!
//! Builds a Tokio runtime, wraps [`tcl_lsp_server::Backend`] in an
//! `LspService`, and serves the LSP protocol over stdio. All
//! decision logic lives in `tcl_lsp_server::Backend` and the
//! pure-crate feature providers — this binary is just the
//! transport, plus one capability-injection shim (below).

#![forbid(unsafe_code)]

use tcl_lsp_server::Backend;
use tower::ServiceExt as _;
use tower_lsp_server::jsonrpc::Response;
use tower_lsp_server::{LspService, Server};

/// Inject `typeHierarchyProvider` into the serialised `initialize` response.
///
/// The type-hierarchy request handlers (`prepare_type_hierarchy` /
/// `supertypes` / `subtypes`) are implemented, but `ls-types` 0.0.6's
/// `ServerCapabilities` struct has no `type_hierarchy_provider` field, so the
/// capability cannot be advertised through the normal typed path.  Dynamic
/// `client/registerCapability` is not an option either: it does not appear in
/// the client's `initializeResult.capabilities`, which editors (and our VS
/// Code test suite) inspect to decide the provider is present.
///
/// So we post-process the response instead: the `initialize` reply is the only
/// one whose result carries a `capabilities` object, so we key off that and add
/// `typeHierarchyProvider: true` (LSP allows a bare boolean here).  Every other
/// response passes through untouched.
fn inject_type_hierarchy_provider(response: Response) -> Response {
    let (id, body) = response.into_parts();
    let Ok(mut result) = body else {
        return Response::from_parts(id, body);
    };
    if let Some(caps) = result
        .get_mut("capabilities")
        .and_then(|c| c.as_object_mut())
        && !caps.contains_key("typeHierarchyProvider")
    {
        caps.insert(
            "typeHierarchyProvider".to_owned(),
            serde_json::Value::Bool(true),
        );
    }
    Response::from_parts(id, Ok(result))
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(Backend::new);
    // Wrap the service so every outgoing response passes through the
    // type-hierarchy capability shim (a no-op for all but `initialize`).
    let service =
        service.map_response(|resp: Option<Response>| resp.map(inject_type_hierarchy_provider));
    Server::new(stdin, stdout, socket).serve(service).await;
}
