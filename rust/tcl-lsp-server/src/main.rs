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
//! transport, plus the two protocol shims in
//! [`tcl_lsp_server::service`].
//!
//! **Native only.** Stdio and a multi-thread Tokio runtime are the two things
//! a wasm32 target does not have, and Cargo builds every `[[bin]]` for
//! whatever target it is given — there is no per-target bin switch — so
//! `cargo check --target wasm32-unknown-unknown -p tcl-lsp-server` compiles
//! this file too. Every item below is therefore gated to non-wasm targets and
//! a bare `main` stands in for wasm, which turns that check into a check of
//! the *library*: the half a browser worker actually links. The browser host
//! is `tcl-lsp-server-wasm`, which drives the same `LspService` over
//! `postMessage`.

#![forbid(unsafe_code)]

#[cfg(not(target_family = "wasm"))]
use tcl_lsp_server::Backend;
#[cfg(not(target_family = "wasm"))]
use tcl_lsp_server::service::{inject_type_hierarchy_provider, normalise_request_uris};
#[cfg(not(target_family = "wasm"))]
use tcl_lsp_server::stdio_pump;
#[cfg(not(target_family = "wasm"))]
use tcl_lsp_server::transport_liveness::{
    DEFAULT_HANDLER_CONCURRENCY, DeferredConcurrency, UNBOUNDED_TRANSPORT_CONCURRENCY,
};
#[cfg(not(target_family = "wasm"))]
use tower::ServiceExt as _;
#[cfg(not(target_family = "wasm"))]
use tower_lsp_server::jsonrpc::Response;
#[cfg(not(target_family = "wasm"))]
use tower_lsp_server::{LspService, Server};

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
#[cfg(not(target_family = "wasm"))]
const WORKER_STACK_SIZE: usize = 64 * 1024 * 1024;

#[cfg(not(target_family = "wasm"))]
fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(WORKER_STACK_SIZE)
        .build()
        .expect("failed to build the Tokio runtime")
        .block_on(serve());
}

#[cfg(not(target_family = "wasm"))]
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
    // Keep stdin routing independent of handler progress. The transport's
    // queue is always drained into pending futures, while the wrapper retains
    // the original four-handler application concurrency for ordinary work.
    // Exit and cancellation remain immediate transport controls.
    let service = DeferredConcurrency::new(service, DEFAULT_HANDLER_CONCURRENCY);
    Server::new(stdin, stdout, socket)
        .concurrency_level(UNBOUNDED_TRANSPORT_CONCURRENCY)
        .serve(service)
        .await;
    // `serve` has returned, so the transport's `FramedWrite` — and with it the
    // pump's sender — has been dropped, which is what lets the drain task
    // finish. Awaiting it here is what stops a burst of diagnostics sitting in
    // the queue from being lost to `main` returning out from under it.
    let _ = stdout_drained.await;
}

/// Stands in for the real `main` on wasm, where there is no stdio to serve
/// over — a `[[bin]]` target still has to have one. The browser host is
/// `tcl-lsp-server-wasm`.
#[cfg(target_family = "wasm")]
fn main() {}
