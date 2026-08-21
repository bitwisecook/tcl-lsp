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
    // Evidence-only #1657 watchdog. The external-spawn experiment falsified
    // its proposed recovery mechanism (zero true resumptions), so normal
    // servers do not pay for or rely on it. An evidence run opts in explicitly.
    if std::env::var_os("TCL_LSP_WEDGE_EVIDENCE").is_some() {
        tcl_lsp_server::spawn_unpark_watchdog(service.inner(), tokio::runtime::Handle::current());
    }
    // Wrap the service so every incoming message passes through the URI
    // canonicalisation shim (a no-op for a conforming client) and every
    // outgoing response through the type-hierarchy capability shim (a no-op for
    // all but `initialize`).
    let service = service
        .map_request(normalise_request_uris)
        .map_response(|resp: Option<Response>| resp.map(inject_type_hierarchy_provider));
    // INVARIANT (ordered diagnostics without a server-wide backpressure blast
    // radius): one persistent publisher owns every diagnostics `Client` await.
    // Producers commit the pull-cache state and a latest-wins per-URI mailbox
    // entry while their document ordering guard is held, then release that
    // guard before awaiting the publisher's receipt. A superseded pending state
    // is settled explicitly; an already-started send is never cancelled or
    // retried (`SinkExt::send` can have completed `start_send` before parking
    // in `poll_flush`, so retrying could duplicate it). The single consumer
    // preserves the cross-URI order of surviving commits and sends a newer
    // same-URI state after any in-flight predecessor.
    //
    // `tower-lsp-server` 0.23 then queues each awaited client message through
    // its bounded `futures::mpsc::channel(1)`, and `stdio_pump` preserves FIFO
    // order to one writer without coupling stdout backpressure to stdin. Re-
    // audit `deliver_diagnostics`, `DiagnosticPublisher`,
    // `deliver_fast_tier_if_current` and `publish_diagnostics_result` if that
    // transport changes. In particular: never add a second diagnostics
    // consumer, never timeout/retry an in-flight send, and never await client
    // I/O while holding `documents` or another request-critical guard.
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
