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
use tcl_lsp_server::stdio_pump;
use tcl_lsp_server::uri_norm::normalise_uris_in_params;
use tower::ServiceExt as _;
use tower_lsp_server::jsonrpc::{Request, Response};
use tower_lsp_server::{LspService, Server};

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
fn normalise_request_uris(request: Request) -> Request {
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

/// Every Tokio worker thread's stack budget.
///
/// The analyser's `analyse_body` recursion (`tcl_compiler::analyser`) and
/// the CFG builder's `lower_script` recursion (`tcl_compiler::cfg_builder`)
/// each bound their nesting depth at a fixed cap (currently 256), but that
/// cap only guarantees a *bounded* number of native stack frames — it says
/// nothing about how much stack those frames need, and that need grows
/// every time a hot function in the chain gains a local variable. Tokio's
/// default worker-thread stack is 2 MiB, well under half of what capped
/// recursion needs even today (measured: a 2 MiB stack overflows around
/// nesting depth 130-140 — see issue #996). Sizing worker threads
/// generously here is the load-bearing fix for the crash; the depth caps
/// alone were never enough on this runtime's actual thread stacks.
const WORKER_STACK_SIZE: usize = 64 * 1024 * 1024;

fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(WORKER_STACK_SIZE)
        .build()
        .expect("failed to build the Tokio runtime")
        .block_on(serve());
}

async fn serve() {
    let stdin = tokio::io::stdin();
    // INVARIANT (no wedged sessions): the transport's write half must never be
    // the reason its read half stops. `tower-lsp-server` 0.23 joins
    // `read_input`, `process_server_tasks` and `print_output` on one task,
    // chained by bounded channels, so a client that stops draining stdout
    // seizes the chain all the way back to the only thing reading stdin — and
    // a server that has stopped reading stdin makes the client block in
    // `write()`, which is why it never resumes draining stdout. That is the
    // 8h45m hang in issue #1334, and the server-wide unresponsiveness in
    // #1294. `stdio_pump::pump` decouples the two halves; its module docs
    // carry the full derivation and the reason the queue has to be unbounded.
    let (stdout, stdout_drained) = stdio_pump::pump(tokio::io::stdout());
    let (service, socket) = LspService::new(Backend::new);
    // Wrap the service so every incoming message passes through the URI
    // canonicalisation shim (a no-op for a conforming client) and every
    // outgoing response through the type-hierarchy capability shim (a no-op for
    // all but `initialize`).
    let service = service
        .map_request(normalise_request_uris)
        .map_response(|resp: Option<Response>| resp.map(inject_type_hierarchy_provider));
    // INVARIANT (no lost and no reordered diagnostics under a slow client):
    // the delivery path needs outbound messages to be *ordered* and *never
    // dropped*. `tower-lsp-server` 0.23 gives the `Client` a bounded
    // `futures::mpsc::channel(1)` drained by a single `.forward(FramedWrite(
    // stdout))`, so a `client.publish_diagnostics(..).await` resolves only
    // once the message is durably queued — it never `try_send`s
    // (drop-on-full). Behind that, `stdio_pump` keeps a single FIFO queue
    // drained by a single writer task, so ordering and no-drop both hold end
    // to end; what it deliberately does not do is make a slow client stall the
    // producer, because that coupling is what deadlocked the session (#1334).
    //
    // Two things must not silently break this — re-audit the whole delivery
    // path (`deliver_diagnostics`, `deliver_fast_tier_if_current`,
    // `publish_diagnostics_result`) if either changes:
    //   1. Swapping `tower-lsp-server` for a build whose client channel uses
    //      `try_send`, or reordering the outbound path so more than one task
    //      writes to the pump — outbound would then drop or interleave. The dep
    //      is version-pinned in `Cargo.toml`; treat an upgrade that touches its
    //      transport as a delivery-review gate.
    //   2. Making any diagnostics publish fire-and-forget (e.g. wrapping it in a
    //      detached `tokio::spawn` to avoid holding the `documents` lock across
    //      the send). Concurrent publishes would reorder, and a
    //      state-gated/disconnected drop would go unnoticed. Delivery MUST stay
    //      an inline `.await`.
    Server::new(stdin, stdout, socket).serve(service).await;
    // `serve` has returned, so the transport's `FramedWrite` — and with it the
    // pump's sender — has been dropped, which is what lets the drain task
    // finish. Awaiting it here is what stops a burst of diagnostics sitting in
    // the queue from being lost to `main` returning out from under it.
    let _ = stdout_drained.await;
}
