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

//! An outbound-stdout adapter that keeps the server reading stdin no matter
//! how slowly the client reads stdout — the fix for issue #1334.
//!
//! # The deadlock this exists to break
//!
//! `tower_lsp_server::Server::serve` joins three futures on one task, chained
//! by bounded channels:
//!
//! ```text
//! read_input ──server_tasks_tx(100)──▶ process_server_tasks
//!                                            │
//!                                     responses_tx(0)
//!                                            ▼
//!                                       print_output ──▶ FramedWrite(stdout)
//! ```
//!
//! Every link is bounded, so backpressure propagates all the way back to
//! `read_input`, which awaits `server_tasks_tx.send(..)`. When the client
//! stops draining stdout the chain seizes in order: the framed write goes
//! `Pending`, `responses_rx` (capacity 0) stops being drained,
//! `process_server_tasks` stalls, the 100-slot task queue fills, and
//! `read_input` — the *only* thing reading stdin — parks on a full channel.
//!
//! The server has now stopped reading stdin while doing no work. The client
//! fills the 64 KiB stdin pipe, blocks in `write()`, and therefore never
//! returns to draining stdout. Both processes sleep forever at 0% CPU, which
//! is exactly the trace in #1334 (and, from the client's side, #1294).
//!
//! There is a second, tighter path to the same place: a handler suspended in
//! `client.publish_diagnostics(..).await` is parked on the client socket's
//! own `channel(1)`, which is *also* drained only by `print_output`. Four such
//! handlers fill `buffer_unordered(DEFAULT_MAX_CONCURRENCY)` and the pipeline
//! seizes without the 100-slot queue ever filling.
//!
//! # Why unbounded is the right shape here
//!
//! Both paths reduce to one property: the server cannot make progress reading
//! stdin unless stdout is draining. Any bound on the outbound path preserves
//! that coupling and merely changes how long the client must stall before the
//! session wedges. Deadlock-freedom requires that outbound messages be
//! absorbed without ever blocking the reader, and LSP does not permit dropping
//! them, so the queue must be unbounded. This is the same shape
//! `rust-analyzer`'s `lsp-server` uses: a dedicated writer with an unbounded
//! queue in front of it.
//!
//! What the original bounded transport was protecting — **no dropped and no
//! reordered diagnostics under a slow client** — is preserved exactly: a
//! single FIFO channel drained by a single writer task keeps delivery ordered,
//! and an unbounded channel never drops. What is given up is the throttling of
//! a *producer* that outruns the client, which was never load-bearing for
//! correctness and was the direct cause of the hang. The cost is memory, and
//! it is observable: see [`BACKLOG_WARN_BYTES`].

use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};

use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

/// Backlog at which the writer complains, once, on stderr.
///
/// Trading a deadlock for unbounded memory is only defensible if the trade is
/// visible when it actually bites, so a client that has stopped reading leaves
/// a trace rather than just a growing RSS. Sized well above any legitimate
/// burst: a full workspace's diagnostics for the largest corpus member is
/// under a megabyte, so reaching this means the client is not reading at all.
pub const BACKLOG_WARN_BYTES: usize = 64 * 1024 * 1024;

/// The write half handed to `tower_lsp_server::Server::new`.
///
/// `poll_write` never returns `Pending`: it moves the bytes into the queue and
/// reports them written. That is the whole point — the transport's writer must
/// never be the reason the reader stops.
#[derive(Debug)]
pub struct PumpedWriter {
    /// `None` after `poll_shutdown`, which closes the channel and lets the
    /// drain task finish.
    tx: Option<UnboundedSender<Vec<u8>>>,
    queued: Arc<AtomicUsize>,
    warned: bool,
}

impl PumpedWriter {
    /// Bytes accepted but not yet written to the real sink.
    #[must_use]
    pub fn backlog(&self) -> usize {
        self.queued.load(Ordering::Relaxed)
    }
}

impl AsyncWrite for PumpedWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let Some(tx) = self.tx.as_ref() else {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "stdout pump already shut down",
            )));
        };
        // Count the bytes *before* handing them over. The drain task subtracts
        // as soon as it has written a chunk, and it runs on another thread, so
        // adding afterwards leaves a window where the subtraction lands first
        // and wraps the counter — after which this arithmetic overflows and
        // panics the transport future, taking the whole server down. Ordering
        // the accounting this way makes the counter monotone by construction.
        let queued = self
            .queued
            .fetch_add(buf.len(), Ordering::Relaxed)
            .saturating_add(buf.len());
        // A send failure means the drain task is gone, which happens when the
        // real stdout errored — the client went away. Report it as a write
        // error so the transport tears the session down as it would have.
        if tx.send(buf.to_vec()).is_err() {
            self.queued.fetch_sub(buf.len(), Ordering::Relaxed);
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "stdout pump drained task has stopped",
            )));
        }
        if queued >= BACKLOG_WARN_BYTES && !self.warned {
            self.warned = true;
            eprintln!(
                "tcl-lsp-server: {queued} bytes of LSP output are queued — the client has stopped \
                 reading stdout. Output is being buffered rather than blocking the server (#1334)."
            );
        }
        Poll::Ready(Ok(buf.len()))
    }

    /// Always ready.
    ///
    /// This does *not* mean the bytes have reached the client, and that is
    /// deliberate: `FramedWrite` flushes after each message, so a flush that
    /// waited for the client would reintroduce precisely the coupling this
    /// type removes. Durability is the drain task's job, and it flushes
    /// whenever the queue empties, so an idle server has always flushed.
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // Dropping the sender is what tells the drain task to flush and exit.
        self.tx = None;
        Poll::Ready(Ok(()))
    }
}

/// Wrap `sink` in a queue drained by a dedicated task.
///
/// Returns the writer to hand to the transport and the drain task's handle.
/// The caller must await the handle *after* the transport has returned (which
/// drops the writer, closing the queue), or buffered output can be lost when
/// the process exits — an `exit` notification arriving right behind a burst of
/// diagnostics is the realistic case.
pub fn pump<W>(sink: W) -> (PumpedWriter, JoinHandle<()>)
where
    W: AsyncWrite + Unpin + Send + 'static,
{
    let (tx, rx) = mpsc::unbounded_channel();
    let queued = Arc::new(AtomicUsize::new(0));
    let handle = tokio::spawn(drain(rx, sink, Arc::clone(&queued)));
    (
        PumpedWriter {
            tx: Some(tx),
            queued,
            warned: false,
        },
        handle,
    )
}

/// Write queued chunks to the real sink, in order, until the queue closes.
async fn drain<W>(mut rx: UnboundedReceiver<Vec<u8>>, mut sink: W, queued: Arc<AtomicUsize>)
where
    W: AsyncWrite + Unpin,
{
    while let Some(chunk) = rx.recv().await {
        let len = chunk.len();
        let wrote = sink.write_all(&chunk).await;
        queued.fetch_sub(len, Ordering::Relaxed);
        if wrote.is_err() {
            // The client's end is gone. Nothing left to deliver to; dropping
            // `rx` makes subsequent `poll_write`s report a broken pipe, which
            // is how the transport learns the session is over.
            return;
        }
        // Coalesce flushes under load, but never leave a message sitting in
        // the buffer once there is nothing behind it — that would look to the
        // client exactly like the hang this module exists to prevent.
        if rx.is_empty() && sink.flush().await.is_err() {
            return;
        }
    }
    let _ = sink.flush().await;
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    /// A sink that never accepts anything, modelling a client that has stopped
    /// reading stdout.
    #[derive(Debug, Default)]
    struct StalledSink;

    impl AsyncWrite for StalledSink {
        fn poll_write(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
            _: &[u8],
        ) -> Poll<io::Result<usize>> {
            Poll::Pending
        }
        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Pending
        }
        fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Pending
        }
    }

    /// A sink that records everything written to it.
    #[derive(Debug, Clone, Default)]
    struct RecordingSink(Arc<Mutex<Vec<u8>>>);

    impl AsyncWrite for RecordingSink {
        fn poll_write(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Poll::Ready(Ok(buf.len()))
        }
        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
        fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    /// The backlog accounting stays sane while the drain task runs
    /// concurrently: writer and drain touch the same counter from different
    /// threads, and it must not wrap.
    ///
    /// What this test is *not*: a regression test for the count-then-send
    /// ordering in `poll_write`. That bug — accounting after handing the chunk
    /// over, letting the drain's subtraction land first, wrapping the counter
    /// and overflow-panicking the transport future — was measured against this
    /// test and it does **not** catch it: the window is a few instructions
    /// wide and was not reachable in 3×20,000 iterations, with or without the
    /// yields. The e2e suite is what caught it, and the guarantee comes from
    /// the ordering itself being monotone by construction. Do not treat a pass
    /// here as cover for changing that ordering.
    ///
    /// `multi_thread` is still required for the concurrency this *does* check:
    /// on the default current-thread runtime the drain cannot run at all while
    /// the writer is running.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn the_backlog_counter_never_wraps_against_a_fast_drain() {
        let sink = RecordingSink::default();
        let (mut writer, drained) = pump(sink);
        for _ in 0..20_000 {
            writer.write_all(b"some outbound bytes").await.unwrap();
            // Hand the scheduler an opening for the drain task between every
            // write, so writer and drain stay interleaved rather than the
            // writer running away with the queue.
            tokio::task::yield_now().await;
            // A wrapped counter shows up as an absurd backlog long before it
            // reaches the arithmetic that panics.
            assert!(
                writer.backlog() < BACKLOG_WARN_BYTES,
                "backlog wrapped: {}",
                writer.backlog()
            );
        }
        writer.shutdown().await.unwrap();
        drop(writer);
        drained.await.unwrap();
    }

    /// The core property: writes complete even when the sink never accepts a
    /// byte. Without this, `FramedWrite` returns `Pending` and the whole
    /// transport chain seizes back to the stdin reader.
    #[tokio::test]
    async fn writes_complete_while_the_sink_is_stalled() {
        let (mut writer, _drain) = pump(StalledSink);
        for i in 0..1_000u32 {
            writer
                .write_all(&i.to_le_bytes())
                .await
                .expect("write must not block on a stalled sink");
        }
        writer.flush().await.expect("flush must not block either");
        assert_eq!(writer.backlog(), 4_000, "nothing should have been written");
    }

    /// Ordering and completeness — the properties the bounded transport was
    /// there to guarantee, which must survive the change.
    #[tokio::test]
    async fn everything_arrives_in_order() {
        let sink = RecordingSink::default();
        let seen = Arc::clone(&sink.0);
        let (mut writer, drained) = pump(sink);

        let mut expected = Vec::new();
        for i in 0..1_000u32 {
            let chunk = format!("msg{i};");
            expected.extend_from_slice(chunk.as_bytes());
            writer.write_all(chunk.as_bytes()).await.unwrap();
        }
        writer.shutdown().await.unwrap();
        drop(writer);
        drained.await.unwrap();

        assert_eq!(*seen.lock().unwrap(), expected);
    }

    /// A dead client must surface as a write error, not a silent stall — the
    /// transport relies on that to tear the session down.
    #[tokio::test]
    async fn a_failed_sink_becomes_a_write_error() {
        #[derive(Debug)]
        struct FailingSink;
        impl AsyncWrite for FailingSink {
            fn poll_write(
                self: Pin<&mut Self>,
                _: &mut Context<'_>,
                _: &[u8],
            ) -> Poll<io::Result<usize>> {
                Poll::Ready(Err(io::Error::from(io::ErrorKind::BrokenPipe)))
            }
            fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
                Poll::Ready(Ok(()))
            }
            fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
                Poll::Ready(Ok(()))
            }
        }

        let (mut writer, _drain) = pump(FailingSink);
        // The first write is absorbed by the queue; the drain task then fails
        // and closes it, so a later write reports the broken pipe.
        writer.write_all(b"first").await.unwrap();
        let mut err = None;
        for _ in 0..100 {
            tokio::task::yield_now().await;
            if let Err(e) = writer.write_all(b"next").await {
                err = Some(e);
                break;
            }
        }
        assert_eq!(
            err.expect("a dead sink must eventually surface as a write error")
                .kind(),
            io::ErrorKind::BrokenPipe
        );
    }
}
