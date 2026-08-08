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

//! Regression test for the stdio deadlock in issue #1334 (client side: #1294).
//!
//! Drives the real `tower_lsp_server::Server` transport against a stdout that
//! never accepts a byte — a client that has stopped reading — and asserts the
//! server keeps draining stdin anyway. The control case runs the same traffic
//! straight at the stalled sink and shows the transport wedging on its own,
//! which is what makes the assertion meaningful rather than vacuous.

use std::convert::Infallible;
use std::future::{Ready, ready};
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};

use futures_util::{sink, stream};
use tokio::io::AsyncRead;
use tower::Service;
use tower_lsp_server::jsonrpc::{Request, Response};
use tower_lsp_server::{Loopback, Server};

use tcl_lsp_server::stdio_pump;

/// Requests fed to the server. Comfortably past both bounds in the transport
/// that can seize: `DEFAULT_MAX_CONCURRENCY` (4) and `MESSAGE_QUEUE_SIZE`
/// (100).
const REQUESTS: usize = 400;

/// How long a healthy server gets to drain `REQUESTS` messages.
const DRAIN_BUDGET: std::time::Duration = std::time::Duration::from_secs(10);

// ---------------------------------------------------------------- test doubles

/// A stdout that never accepts anything: the client has stopped reading.
#[derive(Debug, Default)]
struct StalledSink;

impl tokio::io::AsyncWrite for StalledSink {
    fn poll_write(self: Pin<&mut Self>, _: &mut Context<'_>, _: &[u8]) -> Poll<io::Result<usize>> {
        Poll::Pending
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Pending
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Pending
    }
}

/// A stdin that yields `payload` once and then stays pending, modelling a
/// client that has sent a burst and is still connected. If this returned EOF
/// the server would shut down cleanly and the deadlock could not be observed.
#[derive(Debug)]
struct BurstThenIdle(Vec<u8>);

impl AsyncRead for BurstThenIdle {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.0.is_empty() {
            return Poll::Pending;
        }
        let n = buf.remaining().min(self.0.len());
        let rest = self.0.split_off(n);
        buf.put_slice(&self.0);
        self.0 = rest;
        Poll::Ready(Ok(()))
    }
}

/// Counts how many requests actually reached the service — i.e. how much of
/// stdin the server managed to read.
#[derive(Debug, Clone)]
struct CountingService(Arc<AtomicUsize>);

impl Service<Request> for CountingService {
    type Response = Option<Response>;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        self.0.fetch_add(1, Ordering::SeqCst);
        let (_method, id, _params) = req.into_parts();
        // Answering every request is what puts the outbound path under load;
        // a service that returned `None` would never exercise the stall.
        ready(Ok(id.map(|id| {
            Response::from_parts(id, Ok(serde_json::Value::Null))
        })))
    }
}

/// No server-to-client traffic of its own; the outbound pressure in this test
/// comes from responses.
struct NoLoopback;

impl Loopback for NoLoopback {
    type RequestStream = stream::Pending<Request>;
    type ResponseSink = sink::Drain<Response>;

    fn split(self) -> (Self::RequestStream, Self::ResponseSink) {
        (stream::pending(), sink::drain())
    }
}

// ---------------------------------------------------------------- helpers

/// `REQUESTS` framed JSON-RPC requests, back to back.
fn burst() -> Vec<u8> {
    let mut out = Vec::new();
    for id in 0..REQUESTS {
        let body =
            format!(r#"{{"jsonrpc":"2.0","method":"textDocument/hover","params":{{}},"id":{id}}}"#);
        out.extend_from_slice(format!("Content-Length: {}\r\n\r\n{body}", body.len()).as_bytes());
    }
    out
}

/// Run the transport until it drains `REQUESTS` or `DRAIN_BUDGET` expires,
/// and report how many requests got through.
async fn drained_within_budget<W>(stdout: W) -> usize
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let calls = Arc::new(AtomicUsize::new(0));
    let service = CountingService(Arc::clone(&calls));
    let served = Server::new(BurstThenIdle(burst()), stdout, NoLoopback).serve(service);
    // `serve` never returns here — stdin stays open by construction — so the
    // timeout is the exit path for both the healthy and the wedged case.
    let _ = tokio::time::timeout(DRAIN_BUDGET, served).await;
    calls.load(Ordering::SeqCst)
}

// ---------------------------------------------------------------- the tests

/// The control. Writing straight at a stalled stdout, the transport's own
/// bounded chain seizes back to the stdin reader and the server stops reading
/// long before the burst is consumed. This is the bug in #1334; if this test
/// ever starts draining all `REQUESTS`, the upstream transport has changed and
/// the pump's justification needs re-checking.
#[tokio::test]
async fn unpumped_stdout_wedges_the_stdin_reader() {
    let drained = drained_within_budget(StalledSink).await;
    println!("raw transport drained {drained}/{REQUESTS} before wedging");
    // A lower bound as well as an upper one: draining *nothing* would mean the
    // test wedged for some unrelated reason and would pass for the wrong
    // reason. Where exactly it stops is not worth pinning — it is the 100-slot
    // task queue plus the four `buffer_unordered` slots plus whatever the
    // framed reader had already buffered, which lands around 240 here — the
    // property under test is only that it stops short and never recovers.
    assert!(
        (1..REQUESTS).contains(&drained),
        "expected the raw transport to wedge part-way against a stalled client, \
         got {drained}/{REQUESTS} — re-check the deadlock analysis in \
         stdio_pump's module docs against the current tower-lsp-server"
    );
}

/// The fix. With the pump in front of the same stalled stdout, stdin keeps
/// draining: the server stays responsive to a client that has stopped reading,
/// so the client never blocks in `write()` and the session cannot deadlock.
#[tokio::test]
async fn pumped_stdout_keeps_the_stdin_reader_running() {
    let (stdout, _drain) = stdio_pump::pump(StalledSink);
    let drained = drained_within_budget(stdout).await;
    assert_eq!(
        drained, REQUESTS,
        "the server must keep reading stdin while the client is not reading stdout"
    );
}
