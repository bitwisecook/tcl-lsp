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

//! The two protocol shims every host of this server applies.
//!
//! `LspService<Backend>` is the whole protocol core, and it is host-agnostic —
//! the native binary drives it over stdio, a browser worker drives it over
//! `postMessage`. Two things sit either side of it that are *not* handler
//! logic and that every host needs identically: URIs must be canonicalised on
//! the way in, and the `initialize` reply must advertise a capability the
//! typed `ServerCapabilities` struct cannot express.
//!
//! They live here, in the library, rather than in `main.rs`, so a second host
//! reuses them rather than reimplementing them — a host that skipped
//! [`normalise_request_uris`] would reintroduce issue #1214's split identity
//! for any client that spells a URI differently from the way the server does.
//!
//! Applying them is one line each on the `tower` service:
//!
//! ```ignore
//! let service = service
//!     .map_request(normalise_request_uris)
//!     .map_response(|resp: Option<Response>| resp.map(inject_type_hierarchy_provider));
//! ```

use tower_lsp_server::jsonrpc::{Request, Response};

use crate::uri_norm::normalise_uris_in_params;

/// Put every URI in an incoming message into the one canonical form, before it
/// is deserialised.
///
/// The single boundary at which client-sent URIs meet the ones the server
/// constructs for itself (`tcl_lsp_server::canonical_file_uri`) — issue #1214.
/// Two jobs:
///
/// * **Accept-then-normalise.** Some `JetBrains`- and Neovim-style clients send a
///   folder URI with the spaces left raw. That is not a valid URI, so `Uri`'s
///   `Deserialize` rejects it and the whole `initialize` fails — the session
///   never starts. Repairing it here means such a client is accepted.
/// * **Canonicalise.** A client that upper-cases a Windows drive letter (or
///   lower-cases its percent-escapes) would otherwise spell a file differently
///   from the way the workspace scan spells it, and everything keyed by URI —
///   find-references, workspace symbols, rename — would see one file as two.
///
/// Here rather than in each handler because there is one of it and sixty of
/// them, and because the document store must key on the same spelling a later
/// request looks up. Inert for a conforming client: a VS Code message comes
/// through byte-for-byte unchanged.
#[must_use]
pub fn normalise_request_uris(request: Request) -> Request {
    let (method, id, params) = request.into_parts();
    let params = params.map(|mut p| {
        normalise_uris_in_params(&mut p);
        p
    });
    let builder = Request::build(method);
    let builder = match id {
        Some(id) => builder.id(id),
        None => builder,
    };
    let builder = match params {
        Some(params) => builder.params(params),
        None => builder,
    };
    builder.finish()
}

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
#[must_use]
pub fn inject_type_hierarchy_provider(response: Response) -> Response {
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
