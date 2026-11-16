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

//! Document-sync notifications applied in arrival order, and the barrier requests wait behind.

use crate::{Backend, Uri};

/// How long a request may wait on the document-sync barrier before the server
/// says so on its log channel.
///
/// Well past any honest wait — the barrier covers a buffer splice and a salsa
/// input set, which are milliseconds — and well inside the budgets the editor
/// harness allows a request, so the line lands while the run is still going
/// rather than after it has given up.
pub(in super::super) const EDIT_BARRIER_STALL_WARN: std::time::Duration =
    std::time::Duration::from_secs(10);

/// Greppable marker on the stall line, so a CI log scan can classify a wedged
/// run without reading the prose. Mirrors the harness's own
/// `VSCODE-WAIT-TIMEOUT`.
pub(super) const EDIT_BARRIER_STALL_LOG: &str = "[stall] document-sync barrier has not advanced";

/// Applies document-sync notifications in arrival order.
///
/// `tower_lsp_server` drives incoming messages through
/// `buffer_unordered(max_concurrency)`, which *first-polls* the handler futures
/// in stream order but lets their **awaits** resume in whatever order the
/// runtime schedules. Sequencing on a `Mutex` taken as the first await is
/// therefore not enough: handlers can enter `did_change` as 14,15,16,17 and
/// acquire the lock as 14,16,15,17. An incremental
/// `didChange` is a *range* edit computed against the previous version, so
/// applying one out of order splices it into text it was never computed against
/// and corrupts the buffer permanently — every later feature then reads a
/// document the client never had.
///
/// The ticket is drawn synchronously, before the handler's first `await` — the
/// last point at which arrival order is still known — and each handler then
/// waits for its turn. Request handlers draw no ticket; they only wait for the
/// edits already received ([`EditOrder::settled`]), so read concurrency is
/// unaffected.
///
/// A turn covers the handler's **ordered mutations only** — the buffer splice,
/// the salsa source, the index entry — and is handed on the moment they are
/// committed, not when the handler returns. This is a *global* barrier:
/// every request handler waits on it, so work a handler does after its
/// mutations (a disk reindex, a closed-file republish, a dialect re-scan) must
/// not sit inside the turn, or one document's close stalls every other
/// document's hover.
#[derive(Debug, Default)]
pub(in super::super) struct EditOrder {
    pub(in super::super) next_ticket: std::sync::atomic::AtomicU64,
    pub(in super::super) now_serving: std::sync::atomic::AtomicU64,
    pub(in super::super) advanced: tokio::sync::Notify,
    /// Tickets whose waiter went away before its turn came round.
    ///
    /// A drawn ticket is a reservation in a strictly increasing sequence, so
    /// nothing after it can run until it has been accounted for. If its handler
    /// simply vanishes there is no guard to release it and `now_serving` stops
    /// at that number **for ever** — see [`EditTicket`]. Recording it here lets
    /// [`Self::drain_abandoned`] step over it when the sequence reaches it.
    ///
    /// A `std::sync::Mutex`, deliberately: every critical section touching it is
    /// a few integer operations with no `await` inside, which is exactly the
    /// case a blocking mutex is for. Holding it across an await is what would
    /// make it a hazard, and nothing here does.
    abandoned: std::sync::Mutex<std::collections::BTreeSet<u64>>,
    /// Who currently holds the turn, and since when.
    ///
    /// "The barrier has stopped" is only half a diagnosis — the other half is
    /// *which handler* stopped it, and that cannot be recovered afterwards
    /// because a wedged server writes nothing. Recording it at the grant makes
    /// [`Backend::report_edit_barrier_stall`] able to name the culprit in the
    /// one line it gets out.
    pub(in super::super) holder: std::sync::Mutex<Option<TurnHolder>>,
}

/// The handler currently holding an [`EditOrder`] turn.
#[derive(Debug, Clone)]
pub(in super::super) struct TurnHolder {
    /// The notification that drew the ticket — `didOpen`, `didChange`, `didClose`.
    pub(in super::super) what: &'static str,
    /// The document it is for.
    pub(in super::super) uri: String,
    pub(in super::super) ticket: u64,
    /// `crate::rt::Instant`, never `std::time::Instant`: the latter compiles on
    /// `wasm32-unknown-unknown` and then panics the moment it is called, which
    /// aborts the browser worker and takes the page with it. The shim reads
    /// `performance.now()` there and is a plain re-export of Tokio's natively.
    pub(in super::super) since: crate::rt::Instant,
    /// The `await` the holder last reached, named after the call it is about to
    /// suspend on — see [`EditTurn::at`].
    ///
    /// Naming the *handler* narrows a wedge from "the server" to one
    /// notification, and no further. In a wedge every Tokio worker is parked
    /// with an empty run queue: the holder is neither running nor blocked in
    /// any OS primitive, but suspended at an `await` whose wakeup never
    /// arrived. A suspended future has no thread and therefore appears in no
    /// backtrace, which is why the suspension point has to be recorded on the
    /// way in rather than recovered afterwards.
    pub(in super::super) phase: &'static str,
    /// When [`Self::phase`] was entered. Read against [`Self::since`]: a phase
    /// as old as the turn means the holder never got past its first `await`.
    pub(in super::super) phase_since: crate::rt::Instant,
}

impl EditOrder {
    /// Draw the next ticket. **Must** be called before the handler's first
    /// `await`, while the future is still being first-polled in stream order.
    ///
    /// The returned [`EditTicket`] is a reservation that must be accounted for
    /// however the handler ends — see its docs.
    pub(in super::super) fn take_ticket(&self, what: &'static str, uri: &Uri) -> EditTicket<'_> {
        EditTicket {
            order: self,
            ticket: self
                .next_ticket
                .fetch_add(1, std::sync::atomic::Ordering::AcqRel),
            armed: true,
            what,
            uri: uri.as_str().to_owned(),
        }
    }

    /// Wait for `ticket`'s turn, consuming the reservation and returning the
    /// guard that holds it.
    ///
    /// Cancel-safe in the way the sequence needs: `ticket` is moved into this
    /// future, so a caller dropped while parked here drops the reservation with
    /// it, and [`EditTicket::drop`] records the abandonment rather than leaving
    /// the sequence stuck on a number nobody will ever release.
    pub(in super::super) async fn wait_turn<'a>(
        &'a self,
        mut ticket: EditTicket<'a>,
    ) -> EditTurn<'a> {
        loop {
            let advanced = self.advanced.notified();
            tokio::pin!(advanced);
            // Register before re-reading, so a turn granted between the check and
            // the await cannot be missed.
            advanced.as_mut().enable();
            if self.now_serving.load(std::sync::atomic::Ordering::Acquire) == ticket.ticket {
                // The turn is ours; the guard below now owns the release, so the
                // reservation must not also account for it.
                ticket.armed = false;
                // One reading of the clock for both fields, so "the phase is as
                // old as the turn" — the reading that says the holder never got
                // past its first await — is exact equality rather than a
                // sub-millisecond difference a reader has to squint at.
                let granted = crate::rt::Instant::now();
                *self
                    .holder
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(TurnHolder {
                    what: ticket.what,
                    uri: std::mem::take(&mut ticket.uri),
                    ticket: ticket.ticket,
                    since: granted,
                    phase: TURN_PHASE_GRANTED,
                    phase_since: granted,
                });
                return EditTurn { order: self };
            }
            advanced.await;
        }
    }

    /// Step `now_serving` over any run of consecutive abandoned tickets that
    /// now sits at the head of the queue, and wake whoever that admits.
    ///
    /// Called from both ends: when a turn is handed on normally, and when a
    /// ticket is abandoned. Either can be the event that makes an abandoned
    /// ticket current.
    pub(in super::super) fn drain_abandoned(&self) {
        {
            let mut abandoned = self
                .abandoned
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            while abandoned.remove(&self.now_serving.load(std::sync::atomic::Ordering::Acquire)) {
                self.now_serving
                    .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
            }
        }
        self.advanced.notify_waiters();
    }

    /// The ticket number a barrier taken *now* would have to reach.
    ///
    /// Split out so a caller can snapshot the target, wait on it with a
    /// timeout, and then resume waiting on the **same** target — re-calling
    /// [`Self::settled`] would re-read this and start waiting for edits that
    /// arrived in the meantime, which is a different (and stricter) question.
    pub(in super::super) fn settle_target(&self) -> u64 {
        self.next_ticket.load(std::sync::atomic::Ordering::Acquire)
    }

    /// How far the sequence has actually got.
    pub(in super::super) fn served(&self) -> u64 {
        self.now_serving.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Wait until every edit that had drawn a ticket before `target` has been
    /// applied. Edits arriving later do not hold the caller up, which is why
    /// the target is snapshotted by the caller ([`Self::settle_target`]) rather
    /// than re-read here.
    pub(in super::super) async fn settled_to(&self, target: u64) {
        loop {
            let advanced = self.advanced.notified();
            tokio::pin!(advanced);
            advanced.as_mut().enable();
            if self.now_serving.load(std::sync::atomic::Ordering::Acquire) >= target {
                return;
            }
            advanced.await;
        }
    }
}

/// A drawn but not-yet-granted place in the [`EditOrder`] sequence.
///
/// # Why this is a guard and not a bare number
///
/// A ticket is a reservation in a strictly increasing sequence: `now_serving`
/// advances one at a time, so nothing after ticket N can run until N has been
/// accounted for. [`EditTurn`] accounts for a ticket that was *granted*. Before
/// the grant there is nothing, so a handler future dropped while parked in
/// [`EditOrder::wait_turn`] would leave `now_serving` pointing at a number no
/// guard will ever release.
///
/// The result is not a lost edit but a stopped server. Every later
/// document-sync notification blocks in `wait_turn`, every request handler
/// blocks in [`EditOrder::settled`] and holds one of the four
/// [`crate::transport_liveness::DeferredConcurrency`] permits, and the process
/// sits at zero CPU answering nothing while its stdin reader — which the
/// unbounded transport keeps running — goes on draining input it will never
/// act on. That is the steady state of a wedged server.
///
/// So the reservation itself is the guard. `wait_turn` takes it by value, which
/// puts it inside the very future that might be dropped; if that happens,
/// `Drop` records the abandonment and the sequence steps over it.
pub(in super::super) struct EditTicket<'a> {
    pub(in super::super) order: &'a EditOrder,
    pub(in super::super) ticket: u64,
    /// Still owing an accounting. Cleared when [`EditOrder::wait_turn`] hands
    /// ownership to the [`EditTurn`] it returns.
    pub(in super::super) armed: bool,
    /// Carried so the grant can publish who holds the turn — see
    /// [`EditOrder::holder`].
    pub(in super::super) what: &'static str,
    pub(in super::super) uri: String,
}

impl Drop for EditTicket<'_> {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        {
            let mut abandoned = self
                .order
                .abandoned
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            abandoned.insert(self.ticket);
        }
        // This ticket may be the one the sequence is waiting on right now.
        self.order.drain_abandoned();
    }
}

/// The phase a turn is in between its grant and its first marked `await`.
///
/// Distinct from any call name on purpose: seeing it in a stall line means the
/// holder is stuck in code that draws no marker, which points at the
/// instrumentation before it points at the handler.
pub(in super::super) const TURN_PHASE_GRANTED: &str = "granted (no await reached yet)";

/// A held turn in the [`EditOrder`] sequence; hands it on when dropped.
pub(in super::super) struct EditTurn<'a> {
    pub(in super::super) order: &'a EditOrder,
}

impl EditTurn<'_> {
    /// Record the `await` this turn is about to suspend on.
    ///
    /// Call it **immediately before** the `await`, naming that call: the marker
    /// answers "where did the holder stop", so it has to be in place before the
    /// suspension it is describing, not after the resume that may never come.
    ///
    /// # Why a marker rather than a stack trace
    ///
    /// Such a wedge produces no stack to read. A backtrace of the wedged
    /// process shows every Tokio worker parked with an empty run queue — the
    /// holder is a *suspended future*, owned by no thread, waiting on a wakeup
    /// that never came. Its suspension point exists only as a discriminant
    /// inside a generator on the heap, which no debugger will name. Writing it
    /// down on the way in is the only way to have it afterwards.
    ///
    /// # Cost
    ///
    /// An uncontended `std::sync::Mutex` and two stores, a handful of times per
    /// document-sync notification, on a path that is *already* serialised by the
    /// very lock being taken — the turn holder is unique by construction, so
    /// nothing else can be contending for it. That is orders of magnitude below
    /// the salsa work each marked `await` guards, and it is paid only by
    /// document-sync handlers, never by a request handler's `edits_settled`
    /// fast path.
    pub(in super::super) fn at(&self, phase: &'static str) {
        if let Some(holder) = self
            .order
            .holder
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_mut()
        {
            holder.phase = phase;
            holder.phase_since = crate::rt::Instant::now();
        }
    }
}

impl Drop for EditTurn<'_> {
    fn drop(&mut self) {
        *self
            .order
            .holder
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        self.order
            .now_serving
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        // Handing this turn on may make an abandoned ticket current, in which
        // case the sequence must step over it rather than stop there.
        // `drain_abandoned` wakes the waiters either way.
        self.order.drain_abandoned();
    }
}

impl Backend {
    /// Barrier: block until every document-sync notification the client sent
    /// *before* the calling request has been applied to the document store.
    ///
    /// `tower-lsp-server` drives requests and notifications through one
    /// `buffer_unordered` pool (`transport.rs`), so without this a request
    /// handler can run to completion while an earlier `didChange` is still in
    /// flight and answer from a buffer several edits stale — semantic tokens
    /// whose lines/lengths describe text the client has already replaced, and
    /// positions (hover, completion, definition) resolved against the wrong
    /// offsets. LSP requires a request to observe every notification that
    /// preceded it.
    ///
    /// Waits only for the edits that had already arrived
    /// ([`EditOrder::settled_to`]), so a later edit does not hold this reader up
    /// and readers never serialise against one another.
    ///
    /// A wait here that never ends is the shape of a wedged server — it stops
    /// answering while burning no CPU — and from the outside it is
    /// indistinguishable from any other stall. So a wait that overruns
    /// [`EDIT_BARRIER_STALL_WARN`] says so on the client's log channel before
    /// resuming, which names the barrier as the culprit and says exactly where
    /// the sequence stopped. An extension-host capture quotes the tail of that
    /// channel, so a wedge carries this line with it.
    ///
    /// That reporting must not be paid for on the hot path. **Every** request
    /// handler calls this, and in the overwhelmingly common case there is no
    /// edit in flight at all, so the barrier is already satisfied: the check
    /// below settles it in two atomic loads, allocating nothing and registering
    /// no timer. Only a wait that genuinely has to block reaches the timeout,
    /// and only such a wait builds a `Notified`.
    pub(in super::super) async fn edits_settled(&self) {
        // Snapshot the target so the resumed wait asks the same question the
        // timed-out one did, rather than a stricter one that includes edits
        // which arrived while we waited.
        let target = self.edit_order.settle_target();
        if self.edit_order.served() >= target {
            return;
        }
        crate::transport_liveness::await_outside_request_admission(async {
            if crate::rt::timeout(EDIT_BARRIER_STALL_WARN, self.edit_order.settled_to(target))
                .await
                .is_ok()
            {
                return;
            }
            self.report_edit_barrier_stall(target).await;
            self.edit_order.settled_to(target).await;
        })
        .await;
    }
}
