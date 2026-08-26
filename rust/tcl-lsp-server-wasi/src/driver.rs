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

//! The stdin driver: one thread, one runtime, and a blocking read that must
//! not starve it.
//!
//! # The problem this module exists to solve
//!
//! Every other transport for this server gets its event loop from somewhere
//! else. The native binary has a thread pool: `tokio::io::stdin` parks a
//! dedicated blocking thread while the runtime's workers keep turning. The
//! browser worker has the JS event loop: `postMessage` delivers on it, and
//! `setTimeout` and the microtask queue keep detached work moving between
//! deliveries. wasip1 has neither. There is one thread, there is no reactor
//! for `poll_oneoff` to feed, and a `read` on stdin blocks the whole process —
//! including the Tokio timer wheel and every CPU-ready task the last request
//! detached.
//!
//! That matters because this server *depends* on work continuing after a
//! handler returns. `did_open` returns almost immediately and leaves the
//! analysis detached; the diagnostics it publishes are debounced behind a
//! 50 ms `rt::sleep`. A driver that blocked in `read` after dispatching
//! `did_open` would publish nothing until the client happened to send
//! something else — the client would sit forever waiting for diagnostics that
//! the server had already computed. Equally, `initialized` pulls
//! `workspace/configuration` and awaits the reply, which arrives *on stdin*,
//! so a driver that awaited each handler to completion before reading again
//! would deadlock on the first session it ever ran.
//!
//! # The design
//!
//! One loop, five steps, in this order:
//!
//! 1. **Route** every message the decoder can already make whole. Requests and
//!    notifications are dispatched and immediately *detached*; client replies
//!    to the server's own requests go to the socket's response sink.
//! 2. **Flush** everything the server has queued for the client.
//! 3. **Yield** to the runtime, exactly once, with
//!    [`rt::yield_now`](tcl_lsp_server::rt::yield_now).
//! 4. **Flush** again — whatever step 3 produced goes out before the thread
//!    parks, not after the next client message wakes it.
//! 5. **Wait** in [`wasi_poll::wait_for_stdin`], which blocks the process until
//!    stdin is readable *or* a short deadline passes.
//!
//! Step 3 is what makes step 5 safe, and it is load-bearing in a way that is
//! worth stating precisely. Tokio's `yield_now` defers the driver's waker to
//! the scheduler's deferred list rather than waking it inline, so the
//! current-thread runtime's `block_on` does what it always does with a pending
//! top-level future — runs the ready queue — and then, because a task was
//! deferred, takes its *non-blocking* park (`park_timeout(0)`), which fires
//! every expired timer before polling the driver again. So by the time step 5
//! runs, every CPU-ready task has run and every due timer has *fired* — its
//! waker called, its task queued.
//!
//! One yield is not quite one full drain, and the difference is worth being
//! exact about. `block_on` polls the top-level future — this driver — before
//! the tasks the park just woke, so a timer that fires during step 3's park
//! hands its continuation to a queue that is not serviced until the *next* pass
//! through the loop. A timer's continuation can therefore run up to about two
//! slices after it was due, not one. That is a bound on the continuation, not
//! on the firing, and nothing here needs the tighter one.
//!
//! Step 5's deadline is what keeps the timers moving at all. Tokio's timer
//! wheel only advances when the runtime is driven, and the runtime is not
//! driven while this call is inside the host. The deadline is therefore the
//! dominant term in how late a timer can be — bounding the waiting, though not
//! the inline analysis a pass may run before it reaches the next wait, which on
//! a large document is the larger number. That is why the deadline is short
//! while a session is active and merely modest while it is idle — see
//! [`ACTIVE_SLICE`] and [`IDLE_SLICE`]. Nothing in the server needs better:
//! the tightest deadline it sets is the 50 ms diagnostics debounce, and the
//! longest is the 10 s `workspace/configuration` timeout.
//!
//! # Why not the alternatives
//!
//! *Blocking `read` with a drain first* (no `poll_oneoff` at all) gets step 3's
//! guarantee but loses step 5's: with the thread parked in `read`, a timer that
//! comes due while the client is quiet does not fire until the client speaks,
//! and requirement (3) — the config deadline expiring on an idle session —
//! cannot be met at all.
//!
//! *`tokio::time::sleep` as the wait*, polling stdin non-blockingly around it,
//! inverts the trade: Tokio parks for the true minimum of its own deadlines, so
//! timers are exact, but stdin is then only sampled once per slice and every
//! request pays that latency. Since a late timer is invisible to the client and
//! a late request is not, the wait belongs on stdin.
//!
//! # The one invariant a reader must preserve
//!
//! **The driver must never return `Pending` without having arranged its own
//! wake.** wasip1's `std` has no condvar, so a current-thread runtime that
//! parks with nothing to wake it aborts the process rather than blocking
//! (`condvar wait not supported`). Here that is structural rather than
//! hopeful: the loop's only `.await` is `rt::yield_now()`, which always defers
//! the waker. Every other step is a plain synchronous function, and that is why
//! — [`Self::route`] and [`Self::wait`] would both read naturally as `async`,
//! and are deliberately not. Adding an `.await` on anything that can be pending
//! — a channel receive, a oneshot, `poll_ready` — reintroduces the abort.
//! [`Self::admit`] and [`Self::route`]'s reply queue are written the way they
//! are for exactly this reason.

use std::collections::VecDeque;
use std::io::{BufRead as _, Write as _};
use std::task::Poll;
use std::time::{Duration, Instant};

use futures::StreamExt as _;
use futures::channel::mpsc;
use futures::sink::SinkExt as _;
use tcl_lsp_server::rt;
use tcl_lsp_server::service::{inject_type_hierarchy_provider, normalise_request_uris};
use tower::Service as _;
use tower_lsp_server::jsonrpc::{Request, Response};
use tower_lsp_server::{ClientSocket, LspService};

use crate::framing::{Decoder, encode};
use crate::wasi_poll::{self, Readiness};

/// How long the driver may block on stdin while the session is busy.
///
/// This is the ceiling on a Tokio timer's lateness, so it is set well under the
/// tightest deadline the server keeps (`DIAGNOSTICS_DEBOUNCE`, 50 ms) rather
/// than at some round number.
const ACTIVE_SLICE: Duration = Duration::from_millis(5);

/// How long the driver may block on stdin once a session has gone quiet.
///
/// An idle server wakes ten times a second to do nothing, which is what keeps
/// long deadlines — the 10 s `workspace/configuration` timeout, the 10 s
/// initial-scan wait — firing on an otherwise silent session. Lengthening this
/// trades those deadlines' accuracy for wakeups; shortening it does the
/// reverse.
const IDLE_SLICE: Duration = Duration::from_millis(100);

/// How long after the last byte in either direction the session still counts as
/// active.
///
/// Comfortably longer than the analysis pipeline's own debounces, so a document
/// that is being analysed does not drop to [`IDLE_SLICE`] halfway through.
const ACTIVITY_WINDOW: Duration = Duration::from_millis(500);

/// The most stdin reads the driver will chain before handing the runtime back.
///
/// A client that streams faster than the server analyses must not be able to
/// hold the loop in step 1 forever; 64 reads is far more than a real editor
/// sends in a burst and still bounds the stall at one buffer-fill each.
const MAX_READS_PER_PASS: usize = 64;

/// Why the driver stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// The client sent `exit` after `shutdown`, as the protocol prescribes.
    CleanExit,
    /// The client sent `exit` without a preceding `shutdown`.
    AbruptExit,
    /// Stdin reached end-of-file. The native transport treats a closed stream
    /// the same way — `Server::serve` returns and the process ends normally.
    EndOfInput,
}

impl Stop {
    /// The process exit status this stop reason calls for.
    pub const fn exit_code(self) -> i32 {
        match self {
            Self::CleanExit | Self::EndOfInput => 0,
            Self::AbruptExit => 1,
        }
    }
}

/// The stdin/stdout driver for one session.
pub struct Driver<S> {
    /// The protocol core, wrapped by nothing: the two shims are applied at the
    /// message boundary below, exactly where `main.rs` applies them natively.
    service: LspService<S>,
    /// Client replies to the server's own requests, on their way to the
    /// socket's response sink.
    ///
    /// A channel rather than the sink itself: `tower-lsp-server` 0.23 does not
    /// export `ResponseSink`'s type, so it cannot be named in a struct field,
    /// and routing through a task is the shape the outbound side already uses.
    responses: mpsc::UnboundedSender<Response>,
    /// Everything bound for the client, from both the response side and the
    /// server-request side, already serialised. One queue means one writer,
    /// which means stdout never interleaves two messages.
    outbound_rx: mpsc::UnboundedReceiver<String>,
    /// The handle dispatched calls serialise their replies into.
    outbound_tx: mpsc::UnboundedSender<String>,
    /// Incremental `Content-Length` reader.
    decoder: Decoder,
    /// Requests that arrived while `initialize` was still in flight.
    held: VecDeque<Request>,
    /// When the session last moved a byte in either direction.
    last_activity: Instant,
    /// Whether the client has asked for `shutdown`.
    shutdown_seen: bool,
    /// Set once the host has told us it cannot poll stdin, so the wait degrades
    /// to a plain sleep instead of asking again every pass.
    stdin_pollable: bool,
}

impl<S> Driver<S>
where
    S: tower_lsp_server::LanguageServer,
{
    /// Wire a service and its client socket to stdio.
    pub fn new(service: LspService<S>, socket: ClientSocket) -> Self {
        let (server_requests, response_sink) = socket.split();
        let (outbound_tx, outbound_rx) = mpsc::unbounded::<String>();

        // Client replies to the server's own requests. The sink resolves each
        // against the pending map and never applies backpressure, so this task
        // is a plain relay — it exists only because the sink's type is private.
        let (responses, mut reply_queue) = mpsc::unbounded::<Response>();
        rt::spawn(async move {
            let mut response_sink = response_sink;
            while let Some(response) = reply_queue.next().await {
                if response_sink.send(response).await.is_err() {
                    break;
                }
            }
        });

        // Server→client requests and notifications — diagnostics, log messages,
        // `workspace/configuration`, `client/registerCapability` — join the same
        // queue as the responses. Replies to the requests come back over stdin
        // and are routed to `responses` in `route`.
        let requests_tx = outbound_tx.clone();
        rt::spawn(async move {
            let mut server_requests = server_requests;
            while let Some(request) = server_requests.next().await {
                match serde_json::to_string(&request) {
                    Ok(text) => {
                        if requests_tx.unbounded_send(text).is_err() {
                            break;
                        }
                    }
                    Err(err) => log(&format!("could not serialise a server request: {err}")),
                }
            }
        });

        Self {
            service,
            responses,
            outbound_rx,
            outbound_tx,
            decoder: Decoder::new(),
            held: VecDeque::new(),
            last_activity: Instant::now(),
            shutdown_seen: false,
            stdin_pollable: true,
        }
    }

    /// Run the session to completion.
    pub async fn run(mut self) -> Stop {
        loop {
            // 1. Route everything already decodable, then everything held back
            //    behind `initialize`.
            if let Some(stop) = self.route() {
                self.drain_for_exit().await;
                return stop;
            }
            // 2. Hand the client whatever the server has produced so far.
            self.flush();
            // 3. Let the runtime run every ready task and fire every due timer.
            //    See the module docs: this is what makes step 5 safe.
            rt::yield_now().await;
            // 4. Anything the drain produced goes out before the thread parks.
            self.flush();
            // 5. Park the process — not the runtime, which is now quiescent —
            //    until stdin speaks or the slice expires.
            if self.wait() {
                // End-of-file, but the drain that saw it may have decoded
                // messages in the same pass — `wait` reads before it reports
                // EOF. An `exit` sitting among them decides the process's
                // status, so route once more and honour what that says;
                // end-of-input is only the answer if nothing was waiting.
                let stop = self.route().unwrap_or(Stop::EndOfInput);
                self.drain_for_exit().await;
                return stop;
            }
        }
    }

    /// Dispatch every complete message the decoder holds.
    ///
    /// Returns the stop reason once the client has said `exit`.
    fn route(&mut self) -> Option<Stop> {
        self.admit();
        while let Some(message) = self.decoder.next_message() {
            self.last_activity = Instant::now();
            let text = match message {
                Ok(text) => text,
                Err(err) => {
                    log(&format!("dropped a malformed frame: {err}"));
                    continue;
                }
            };
            match classify(&text) {
                Some(Incoming::Request(request)) => {
                    let method = request.method().to_owned();
                    if method == "shutdown" {
                        self.shutdown_seen = true;
                    }
                    self.held.push_back(*request);
                    self.admit();
                    if method == "exit" {
                        return Some(if self.shutdown_seen {
                            Stop::CleanExit
                        } else {
                            Stop::AbruptExit
                        });
                    }
                }
                Some(Incoming::Response(response)) => {
                    // The reply to a request the *server* made. Queued, not
                    // awaited: an unbounded send cannot suspend the driver, and
                    // the relay task resolves it against the pending map on the
                    // very next scheduler pass — which step 3 always runs.
                    if self.responses.unbounded_send(response).is_err() {
                        return Some(Stop::EndOfInput);
                    }
                }
                None => log(&format!(
                    "dropped a message that is neither a request nor a response: {}",
                    text.chars().take(120).collect::<String>()
                )),
            }
        }
        None
    }

    /// Dispatch as many held requests as the service will currently admit.
    ///
    /// `poll_ready` is `Pending` for as long as an `initialize` is in flight;
    /// that is the transport's way of holding everything else behind it. The
    /// native and browser transports *await* that readiness, which stops them
    /// reading stdin until `initialize` answers. Here the poll is a single
    /// non-suspending probe and anything not yet admissible stays queued, so
    /// stdin keeps routing throughout initialisation — the same independence
    /// the native transport's `DeferredConcurrency` provides, reached a
    /// different way. It also keeps the module invariant intact: an `.await` on
    /// `poll_ready` is an await that can be pending without a self-wake.
    fn admit(&mut self) {
        while !self.held.is_empty() {
            // A no-op waker is correct here precisely because nothing depends
            // on being woken: the loop probes again on its next pass, at most
            // one slice later.
            let waker = std::task::Waker::noop();
            let mut cx = std::task::Context::from_waker(waker);
            match self.service.poll_ready(&mut cx) {
                Poll::Pending => return,
                Poll::Ready(Err(_)) => {
                    // The service has exited; anything still held is moot.
                    self.held.clear();
                    return;
                }
                Poll::Ready(Ok(())) => {}
            }
            let Some(request) = self.held.pop_front() else {
                return;
            };
            let call = self.service.call(normalise_request_uris(request));
            let tx = self.outbound_tx.clone();
            // Detached, always. A handler that awaited a client reply — which
            // `initialized` does — would otherwise be waiting on a message the
            // driver has stopped listening for.
            rt::spawn(async move {
                if let Ok(Some(response)) = call.await {
                    match serde_json::to_string(&inject_type_hierarchy_provider(response)) {
                        Ok(text) => {
                            let _ = tx.unbounded_send(text);
                        }
                        Err(err) => log(&format!("could not serialise a response: {err}")),
                    }
                }
            });
        }
    }

    /// Write every queued outbound message, framed.
    fn flush(&mut self) {
        let mut wrote = false;
        let mut out = std::io::stdout().lock();
        while let Ok(text) = self.outbound_rx.try_recv() {
            if let Err(err) = out.write_all(&encode(&text)) {
                log(&format!("could not write to stdout: {err}"));
                return;
            }
            wrote = true;
        }
        if wrote {
            if let Err(err) = out.flush() {
                log(&format!("could not flush stdout: {err}"));
            }
            self.last_activity = Instant::now();
        }
    }

    /// Block until stdin speaks or the slice expires, then read what is there.
    ///
    /// Returns whether stdin reached end-of-file.
    fn wait(&mut self) -> bool {
        let slice = if self.last_activity.elapsed() < ACTIVITY_WINDOW {
            ACTIVE_SLICE
        } else {
            IDLE_SLICE
        };
        if !self.stdin_pollable {
            // Degraded host: no `fd_read` subscription to wait on, so sleep the
            // slice and then read.
            //
            // Be clear about what this costs, because it is not a small
            // degradation. `fill_buf` on wasip1 stdin *blocks*, so once the
            // sleep is over this thread is parked in `read` until the client
            // sends something. On such a host the loop is the naive blocking
            // driver: requests wait for the next client byte, and so do the
            // runtime's timers — a debounce or a deadline that comes due while
            // stdin is quiet does not fire until the client speaks again. The
            // sleep only bounds how long the *first* pass takes to notice
            // bytes that were already waiting; it cannot bound anything else.
            //
            // This is a floor that keeps a host we cannot poll usable at all,
            // not a second correct mode. wasmtime is not such a host, and no
            // host tcl-lsp currently ships against is.
            wasi_poll::sleep(slice);
            return self.read_available();
        }
        match wasi_poll::wait_for_stdin(slice) {
            Readiness::TimedOut => false,
            Readiness::Readable => self.drain_stdin(),
            Readiness::Unsupported => {
                log(
                    "this host cannot poll stdin for readiness; falling back to timed reads. \
                     Request latency will be up to one poll slice.",
                );
                self.stdin_pollable = false;
                false
            }
        }
    }

    /// Read every chunk stdin has ready, up to [`MAX_READS_PER_PASS`].
    ///
    /// Returns whether end-of-file was reached.
    fn drain_stdin(&mut self) -> bool {
        for _ in 0..MAX_READS_PER_PASS {
            if self.read_available() {
                return true;
            }
            if wasi_poll::wait_for_stdin(Duration::ZERO) != Readiness::Readable {
                return false;
            }
        }
        false
    }

    /// Read one buffer's worth of stdin into the decoder.
    ///
    /// Returns whether end-of-file was reached. Uses `fill_buf`/`consume` and
    /// always consumes the whole buffer, so `std`'s own buffering never holds
    /// bytes that the readiness poll cannot see.
    fn read_available(&mut self) -> bool {
        let mut stdin = std::io::stdin().lock();
        let taken = match stdin.fill_buf() {
            Ok(chunk) => {
                if chunk.is_empty() {
                    return true;
                }
                self.decoder.push(chunk);
                chunk.len()
            }
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => return false,
            Err(err) => {
                log(&format!("could not read stdin: {err}"));
                return true;
            }
        };
        stdin.consume(taken);
        self.last_activity = Instant::now();
        false
    }

    /// Give the server a last chance to answer before the process ends.
    ///
    /// `exit` arrives with a `shutdown` response possibly still in flight, and
    /// with whatever the last notification detached still queued. A few yields
    /// let those finish and reach stdout; they cannot block, because every one
    /// of them is either CPU-ready or already gone.
    async fn drain_for_exit(&mut self) {
        for _ in 0..8 {
            rt::yield_now().await;
            self.flush();
        }
    }
}

/// What a client→server message turned out to be.
enum Incoming {
    /// A request or notification for the server to handle.
    Request(Box<Request>),
    /// The client's reply to a request the *server* made.
    Response(Response),
}

/// Decide whether a message is for the service or for the client socket.
///
/// A JSON-RPC message carrying `method` is a request or a notification and
/// belongs to the service; one carrying `result` or `error` instead is a reply
/// to something the server asked, and belongs to the socket's response sink.
/// Deserialising into `Request` first and falling back would misfile a
/// malformed request as a response, so the discriminator is checked directly.
/// (Identical to the browser transport's `classify`, and deliberately so —
/// the routing rule is the protocol's, not the transport's.)
fn classify(text: &str) -> Option<Incoming> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    if value.get("method").is_some() {
        serde_json::from_value(value)
            .ok()
            .map(|request| Incoming::Request(Box::new(request)))
    } else {
        serde_json::from_value(value).ok().map(Incoming::Response)
    }
}

/// Write one diagnostic line to stderr.
///
/// stderr is the only channel a WASI host leaves free: stdout carries the
/// protocol, and there is no console to reach for.
fn log(message: &str) {
    eprintln!("tcl-lsp-server-wasi: {message}");
}
