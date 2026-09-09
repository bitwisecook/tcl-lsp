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

//! A last-resort process exit for a session that is over but whose
//! `tower-lsp-server` transport has not returned — issue #2021.
//!
//! `Server::serve` (see the dependency's `src/transport.rs`) only returns
//! once *every* in-flight handler future has completed, however the session
//! ended: stdin reaching EOF, or the `exit` notification arriving. Two things
//! can pin a handler future open well past that point and have done, in
//! practice:
//!
//! * `initialized` runs the whole workspace scan inline, and nothing cancels
//!   it on `shutdown`/`exit`/EOF/a broken pipe — an analyser case with
//!   effectively unbounded runtime turned that into an 18-hour-old orphan.
//! * A server-to-client request awaited after the client is gone: nothing
//!   ever fires the pending-response `oneshot` for a reply that will never
//!   arrive (see [`crate::bounded_client_request`], which now bounds every
//!   such site).
//!
//! Bounding those awaits closes most of the gap, but a *third* case is
//! unbounded in principle — a `spawn_blocking` closure already running when
//! the session ends, which `Runtime::drop` blocks on to completion rather
//! than cancelling. `serve` returning is therefore not a guarantee this
//! process can make on its own once the client is gone; the watchdog here is
//! what makes it one anyway.
//!
//! [`ExitSignal`] is fed from two places — [`EofSignalingReader`] wrapping
//! `stdin`, and [`ExitSignal::observe_request`] plugged into the transport's
//! `map_request` chain — and [`ExitSignal::spawn`] starts the backstop task
//! that turns either signal into a bounded-grace [`std::process::exit`].
//! The **normal exit path always wins when it finishes first**: `serve`
//! returning, `stdout_drained.await`, and `main` returning happen exactly as
//! before, this task merely also being spawned changes nothing about them.
//! The watchdog only matters when that path does not finish inside the grace
//! period, which is precisely the case it exists to catch.

use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::io::{AsyncRead, ReadBuf};
use tower_lsp_server::jsonrpc::Request;

/// Grace period the watchdog waits, once the session is over, before it
/// force-exits. Overridable so the e2e suite can pin it to something far
/// shorter than the production default.
///
/// Read once per [`ExitSignal::spawn`] call rather than cached in a static:
/// this is cheap, only ever called once per process, and a plain read keeps
/// the test suite free of process-wide environment-variable ordering
/// concerns.
fn grace_period() -> Duration {
    std::env::var("TCL_LSP_EXIT_GRACE_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map_or(DEFAULT_EXIT_GRACE, Duration::from_millis)
}

/// The production default grace period: long enough that an ordinary,
/// prompt `serve` return never races it, short enough that an orphaned
/// session cannot outlive it by hours.
const DEFAULT_EXIT_GRACE: Duration = Duration::from_millis(3_000);

/// Shared state fed by [`EofSignalingReader`] and [`ExitSignal::observe_request`],
/// and read back by the watchdog task [`ExitSignal::spawn`] starts.
///
/// Cheaply `Clone`: every field is an `Arc`, so every clone observes the same
/// underlying signal.
#[derive(Debug, Clone)]
pub struct ExitSignal {
    /// Fires once the session is over, however that was learned.
    session_over: Arc<tokio::sync::Notify>,
    /// Whether `shutdown` was seen before the session ended — the LSP spec's
    /// distinction between a clean `exit` (code 0) and an unclean one
    /// (code 1).
    shutdown_seen: Arc<AtomicBool>,
    /// Guards `session_over` against firing its permit more than once; a
    /// second EOF poll or a stray repeated `exit` notification must not
    /// queue up extra wakes the single watchdog waiter never consumes.
    fired: Arc<AtomicBool>,
}

impl Default for ExitSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl ExitSignal {
    /// A fresh signal, observed by nothing yet.
    #[must_use]
    pub fn new() -> Self {
        Self {
            session_over: Arc::new(tokio::sync::Notify::new()),
            shutdown_seen: Arc::new(AtomicBool::new(false)),
            fired: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Record that the session is over — stdin EOF or the `exit`
    /// notification, whichever is observed first — and arm the watchdog's
    /// grace timer.
    ///
    /// `Notify::notify_one` (not `notify_waiters`) is what makes the ordering
    /// between this call and [`Self::spawn`]'s `.notified().await` not
    /// matter: a `notify_one` that lands before anyone is waiting is
    /// remembered as a permit and consumed by the next `.notified().await`,
    /// so a session that ends before the watchdog task is even polled for
    /// the first time still wakes it promptly.
    pub fn record_session_over(&self) {
        if !self.fired.swap(true, Ordering::AcqRel) {
            self.session_over.notify_one();
        }
    }

    /// Record that a `shutdown` request was seen, so a later `exit`/EOF
    /// reports the clean exit code.
    pub fn record_shutdown(&self) {
        self.shutdown_seen.store(true, Ordering::Release);
    }

    /// A `tower::Service` `map_request` step: watches every inbound message
    /// for `shutdown` and `exit` and records them, passing everything else
    /// through unchanged. Install this alongside the crate's other
    /// `map_request` shims (see [`crate::service`]).
    #[must_use]
    pub fn observe_request(&self, request: Request) -> Request {
        match request.method() {
            "shutdown" => self.record_shutdown(),
            "exit" => self.record_session_over(),
            _ => {}
        }
        request
    }

    /// Start the backstop task: wait for the session to end, sleep out the
    /// grace period, then hard-exit.
    ///
    /// Exit code follows the LSP spec's `exit` notification: `0` if
    /// `shutdown` was received first, `1` otherwise. This never runs at all
    /// on the path that matters most — `serve` returning, `stdout_drained`
    /// draining, and `main` returning on its own — because that unwinds the
    /// runtime (dropping this detached task, mid-sleep, without consequence)
    /// well before the grace period would elapse in practice. It exists
    /// purely for when that path does not finish in time.
    #[must_use]
    pub fn spawn(&self) -> tokio::task::JoinHandle<()> {
        let signal = self.clone();
        tokio::spawn(async move {
            signal.session_over.notified().await;
            tokio::time::sleep(grace_period()).await;
            let code = i32::from(!signal.shutdown_seen.load(Ordering::Acquire));
            std::process::exit(code);
        })
    }
}

/// Wraps an [`AsyncRead`] so an observed zero-byte read (EOF, by the trait's
/// contract) reports itself to an [`ExitSignal`] — the second of the two
/// session-over signals `main.rs` wires up, alongside the `exit`
/// notification [`ExitSignal::observe_request`] watches for.
///
/// A conforming `AsyncRead::poll_read` signals EOF by returning
/// `Poll::Ready(Ok(()))` having appended zero bytes to `buf`, which is what
/// this adapter checks for rather than inspecting the byte count `poll_read`
/// does not otherwise expose. It is a pure pass-through in every other
/// respect — same bytes, same errors, same `Pending`s — so wrapping `stdin`
/// with it changes nothing about the protocol the transport reads.
#[derive(Debug)]
pub struct EofSignalingReader<R> {
    inner: R,
    signal: ExitSignal,
}

impl<R> EofSignalingReader<R> {
    /// Wrap `inner`, reporting its first zero-byte read to `signal`.
    pub fn new(inner: R, signal: ExitSignal) -> Self {
        Self { inner, signal }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for EofSignalingReader<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let before = buf.filled().len();
        let poll = Pin::new(&mut this.inner).poll_read(cx, buf);
        if matches!(poll, Poll::Ready(Ok(()))) && buf.filled().len() == before {
            this.signal.record_session_over();
        }
        poll
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::io::AsyncReadExt as _;

    use super::*;

    /// Reading every byte out of the source and then past its end reports
    /// exactly one session-over signal, on the read that actually hits EOF —
    /// not on any of the reads that still returned data.
    #[tokio::test]
    async fn eof_reader_signals_only_on_the_zero_byte_read() {
        let signal = ExitSignal::new();
        let mut reader = EofSignalingReader::new(&b"hi"[..], signal.clone());

        // `session_over` has no waiter yet, so poll `fired` directly rather
        // than racing a `.notified()` call against these reads.
        let mut buf = [0_u8; 8];
        let n = reader.read(&mut buf).await.expect("read must succeed");
        assert_eq!(&buf[..n], b"hi", "the real bytes must still come through");
        assert!(
            !signal.fired.load(Ordering::Acquire),
            "a read that returned data must not signal EOF"
        );

        let n = reader.read(&mut buf).await.expect("EOF read must succeed");
        assert_eq!(n, 0, "the exhausted source must report EOF as zero bytes");
        assert!(
            signal.fired.load(Ordering::Acquire),
            "the zero-byte read must signal EOF"
        );
    }

    /// A second EOF read (a caller that keeps polling past end-of-stream, as
    /// `tower-lsp-server`'s framed reader may) must not queue a second
    /// permit the single watchdog waiter never consumes.
    #[tokio::test]
    async fn repeated_eof_reads_signal_only_once() {
        let signal = ExitSignal::new();
        let mut reader = EofSignalingReader::new(&b""[..], signal.clone());
        let mut buf = [0_u8; 8];

        for _ in 0..3 {
            let n = reader.read(&mut buf).await.expect("EOF read must succeed");
            assert_eq!(n, 0);
        }

        // One permit was recorded; draining it once must not leave a second
        // one behind for a waiter that arrives later.
        signal.session_over.notified().await;
        let woke_again =
            tokio::time::timeout(Duration::from_millis(50), signal.session_over.notified()).await;
        assert!(
            woke_again.is_err(),
            "three EOF reads must leave exactly one permit, not three"
        );
    }

    /// [`ExitSignal::observe_request`] is the `map_request` half of the
    /// signal: it must recognise `shutdown` and `exit` and leave every other
    /// method alone.
    #[tokio::test]
    async fn observe_request_tracks_shutdown_then_exit() {
        let signal = ExitSignal::new();

        let request = signal.observe_request(Request::build("initialize").id(0).finish());
        assert_eq!(request.method(), "initialize");
        assert!(!signal.shutdown_seen.load(Ordering::Acquire));
        assert!(!signal.fired.load(Ordering::Acquire));

        let _ = signal.observe_request(Request::build("shutdown").id(1).finish());
        assert!(
            signal.shutdown_seen.load(Ordering::Acquire),
            "shutdown must be recorded"
        );
        assert!(
            !signal.fired.load(Ordering::Acquire),
            "shutdown alone must not end the session"
        );

        let _ = signal.observe_request(Request::build("exit").finish());
        assert!(
            signal.fired.load(Ordering::Acquire),
            "exit must end the session"
        );
        signal.session_over.notified().await;
    }

    /// `exit` with no prior `shutdown` still ends the session — the watchdog
    /// is what turns this into exit code 1, this only checks the signal
    /// itself is raised.
    #[tokio::test]
    async fn observe_request_tracks_exit_without_shutdown() {
        let signal = ExitSignal::new();
        let _ = signal.observe_request(Request::build("exit").finish());
        assert!(signal.fired.load(Ordering::Acquire));
        assert!(!signal.shutdown_seen.load(Ordering::Acquire));
    }
}
