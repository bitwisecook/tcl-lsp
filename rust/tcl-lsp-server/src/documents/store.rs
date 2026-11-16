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

//! The open-document map with its holder record and waiter telemetry.

use super::{
    Arc, DocumentState, DocumentsContention, Future, HashMap, HeldFor, Mutex, PhaseSpan, Pin, Poll,
    Uri, WaiterSnapshot,
};

/// The open-document map, plus a record of **who currently holds it**.
///
/// # Why the map needs a name attached
///
/// A whole server answering nothing is produced by this chain: some task holds
/// this lock and does not give it back → the document-sync handler holding the
/// [`EditOrder`] turn blocks on it → every request handler blocks on the
/// barrier behind that turn. The phase marker on [`TurnHolder`] names only the
/// *waiter* — `did_change: documents.lock`, with the phase as old as the turn.
///
/// Inferring the holder from adjacent log lines is not a name, and a wedge need
/// not leave a tidy trail. This records it directly, so the stall line states
/// the holder instead of implying it.
///
/// # Cost
///
/// One uncontended `std::sync::Mutex` and two stores per acquisition of a lock
/// that is already a serialisation point — the tag is written by whoever just
/// won the tokio mutex, so nothing else can be contending for the tag. Cleared
/// on guard drop.
pub(in super::super) struct DocumentStore {
    pub(in super::super) docs: Mutex<HashMap<Uri, DocumentState>>,
    /// Who holds the map, who held it last, and how many times it has been
    /// taken — under **one** lock, deliberately.
    ///
    /// Split across separate fields (two mutexes and an atomic), a snapshot
    /// built from them could interleave: read the holder, watch that guard drop
    /// and another task acquire, then read the last-holder and the count from
    /// *after* that change. The result is a stall line pairing one task's
    /// identity with another's metadata — "old task still holds", or
    /// free-while-held — which is precisely the confusion the discriminator
    /// exists to remove.
    ///
    /// A single lock makes the whole snapshot atomic rather than merely
    /// contemporaneous. Dating every age from one clock reading fixes when the
    /// numbers were taken; it cannot fix which state they describe.
    ///
    /// It is also cheap: acquiring the map takes one uncontended
    /// `std::sync::Mutex` rather than a mutex plus an atomic RMW.
    pub(in super::super) tracking: std::sync::Mutex<DocumentsTracking>,
}

/// The bookkeeping [`DocumentStore::tracking`] guards. See its docs for why
/// these three live together.
#[derive(Debug, Default)]
pub(in super::super) struct DocumentsTracking {
    /// `None` whenever the map is free.
    pub(in super::super) holder: Option<DocumentsHolder>,
    /// The previous holder, kept after it releases, so a stall line that finds
    /// the map free can still say who had it last.
    pub(in super::super) last: Option<DocumentsHolder>,
    /// Total successful acquisitions, ever.
    ///
    /// The reading that separates the two stories a "free map, waiting holder"
    /// stall leaves open. Sampled twice a moment apart:
    ///
    /// * **Climbing** — other tasks are taking and releasing the map while the
    ///   turn holder waits on it. `tokio::sync::Mutex` is FIFO-fair, so a waiter
    ///   cannot be skipped for twenty seconds; if it is being skipped, its
    ///   acquire future is not in the queue at all, which points at the future
    ///   never being polled rather than at the lock.
    /// * **Frozen** — nothing wants the map, and a waiter that *is* queued was
    ///   never woken. That is a lost wakeup in the ordinary sense.
    pub(in super::super) acquisitions: u64,
    /// Every task currently parked in [`DocumentStore::lock`]'s contended slow
    /// path, with its poll/wake telemetry.
    ///
    /// Registered on entering the slow path, removed by a drop guard when the
    /// acquire completes **or is cancelled** — a `did_close` future dropped
    /// mid-wait must not leave a ghost waiter for the stall line to accuse.
    /// Each entry is its own `Arc<Mutex<..>>` so the per-poll updates never
    /// touch this tracking lock.
    pub(in super::super) waiters: Vec<Arc<std::sync::Mutex<WaiterTelemetry>>>,
}

/// Poll/wake telemetry for one task parked on the open-document map.
///
/// # What this discriminates
///
/// A wedge can leave the map **free** with its acquisition count frozen while
/// `did_open` sits parked on `documents.lock()` — a waiter that was never
/// resumed. Two mechanisms produce that and they need different fixes:
///
/// * **The wake was lost.** The waiter's waker was never invoked: nothing told
///   the task to run again. `wakes` stays at zero after the park.
/// * **The wake arrived and nothing polled.** The waker fired — tokio's mutex
///   handed the lock over — but the future was never polled afterwards. In
///   this server every handler future is a child of the single
///   `buffer_unordered` set inside `Server::serve`, so a child is only ever
///   re-polled when that parent task runs; a fired wake with no subsequent
///   poll says the parent (or the `FuturesUnordered` bookkeeping between them)
///   stopped driving this child. `last_wake` newer than `last_poll`.
///
/// The stall line prints both counters and both ages so the reader gets the
/// verdict, not the ambiguity.
#[derive(Debug)]
pub(in super::super) struct WaiterTelemetry {
    /// The site passed to [`DocumentStore::lock`].
    pub(in super::super) site: &'static str,
    /// When the slow path was entered. `crate::rt::Instant`, never
    /// `std::time::Instant` (the latter panics on wasm32).
    pub(in super::super) parked_since: crate::rt::Instant,
    /// Times the acquire future has been polled.
    pub(in super::super) polls: u64,
    pub(in super::super) last_poll: crate::rt::Instant,
    /// Times this waiter's waker has been invoked.
    pub(in super::super) wakes: u64,
    pub(in super::super) last_wake: Option<crate::rt::Instant>,
    /// Exact Tokio task waiting on this acquire. Native evidence only.
    #[cfg(not(target_family = "wasm"))]
    pub(in super::super) task_id: Option<tokio::task::Id>,
}

/// Wraps the real waker so an invocation is recorded before it is forwarded.
///
/// The record-then-forward order matters: recording after would let the woken
/// task run, poll, and be sampled between the two writes, producing a poll
/// with no wake that explains it.
struct TelemetryWaker {
    real: std::task::Waker,
    telemetry: Arc<std::sync::Mutex<WaiterTelemetry>>,
}

impl TelemetryWaker {
    pub(in super::super) fn record(&self) {
        let mut t = self
            .telemetry
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        t.wakes += 1;
        t.last_wake = Some(crate::rt::Instant::now());
    }
}

impl std::task::Wake for TelemetryWaker {
    fn wake(self: Arc<Self>) {
        self.record();
        self.real.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.record();
        self.real.wake_by_ref();
    }
}

/// Who holds [`DocumentStore::docs`], and since when.
#[derive(Debug, Clone, Copy)]
pub(in super::super) struct DocumentsHolder {
    /// The call site that took it, as passed to [`DocumentStore::lock`], then
    /// whatever [`DocumentStore::retag_held`] most recently renamed it to.
    pub(in super::super) site: &'static str,
    /// When the map was acquired — the age of the **whole** hold, deliberately
    /// not reset by a retag.
    ///
    /// `crate::rt::Instant`, never `std::time::Instant`: the latter compiles on
    /// `wasm32-unknown-unknown` and panics the moment it is called.
    pub(in super::super) since: crate::rt::Instant,
    /// When [`Self::site`] was last set — the age of the **current phase**.
    ///
    /// Without this, a long hold reports only where it currently is and how long
    /// it has held in total, which cannot distinguish "spent a minute in an
    /// earlier phase and just arrived here" from "has been stuck here the whole
    /// time": a retag overwrites the site and leaves no trace of the phase
    /// before it. [`TurnHolder`] carries the same pairing.
    pub(in super::super) phase_since: crate::rt::Instant,
    /// The longest phase this hold has finished, and where it was.
    ///
    /// `site` + `phase_since` answer "is *this* step the slow one", which is
    /// only ever about the phase running right now. A hold that spent a minute
    /// in an earlier phase and then moved on reports a young `in_phase` and a
    /// large `held`, and nothing says which of the phases behind it burned the
    /// time — the retag overwrites the site. Without this field, a hold blamed
    /// on a phase whose own budget is far smaller than the reported age is
    /// really being blamed for time spent in a phase already relabelled away.
    ///
    /// Recorded on every retag *and* on release, because a hold whose slowest
    /// phase is its last one never retags again — without the release update
    /// the very shape most worth catching would be the one that leaves no
    /// trace. `None` until the first phase ends.
    pub(in super::super) longest_phase: Option<PhaseSpan>,
}

impl DocumentsHolder {
    /// Fold the phase ending at `now` into [`Self::longest_phase`].
    ///
    /// Called when the phase is relabelled and again when the hold is released,
    /// so the last phase counts like every other one.
    fn close_phase(&mut self, now: crate::rt::Instant) {
        let elapsed = now.duration_since(self.phase_since);
        if self
            .longest_phase
            .is_none_or(|(_, longest)| elapsed > longest)
        {
            self.longest_phase = Some((self.site, elapsed));
        }
    }
}

impl Default for DocumentStore {
    fn default() -> Self {
        Self {
            docs: Mutex::new(HashMap::new()),
            tracking: std::sync::Mutex::new(DocumentsTracking::default()),
        }
    }
}

impl DocumentStore {
    /// Take the open-document map, recording `site` as the holder for as long
    /// as the returned guard lives.
    ///
    /// `site` is a `&'static str` naming the caller — it ends up in a stall
    /// line a human reads, so name the function, not the file and line.
    pub(in super::super) async fn lock(&self, site: &'static str) -> DocumentsGuard<'_> {
        // Uncontended fast path: no waiter, so nothing to instrument, and the
        // poll/wake shim's per-poll waker allocation is never paid. tokio's
        // mutex is fair, so `try_lock` cannot barge past a queued waiter — a
        // released permit is handed to the queue head, not left for grabs.
        let docs = match self.docs.try_lock() {
            Ok(docs) => docs,
            Err(_) => self.lock_contended(site).await,
        };
        self.record_acquisition(docs, site)
    }

    /// Try to take the open-document map without joining its waiter queue.
    ///
    /// Background index publication uses this only after it has optimistically
    /// acquired every downstream write lock.  Failing instead of waiting is
    /// what prevents that low-priority work from inverting the foreground
    /// `documents -> db -> workspace_index` order used by document sync.
    pub(in super::super) fn try_lock(&self, site: &'static str) -> Option<DocumentsGuard<'_>> {
        self.docs
            .try_lock()
            .ok()
            .map(|docs| self.record_acquisition(docs, site))
    }

    /// Attach holder telemetry to an already-acquired map guard.
    fn record_acquisition<'a>(
        &'a self,
        docs: tokio::sync::MutexGuard<'a, HashMap<Uri, DocumentState>>,
        site: &'static str,
    ) -> DocumentsGuard<'a> {
        // One clock reading for both ages, so a hold still in its first phase
        // reports them equal rather than a sub-millisecond difference to squint
        // at. Count and holder are set under the same lock, so no reader can
        // see one without the other.
        let acquired = crate::rt::Instant::now();
        let mut tracking = self.tracking();
        tracking.acquisitions += 1;
        tracking.holder = Some(DocumentsHolder {
            site,
            since: acquired,
            phase_since: acquired,
            longest_phase: None,
        });
        drop(tracking);
        DocumentsGuard { docs, store: self }
    }

    /// The contended acquire, instrumented.
    ///
    /// Registers a [`WaiterTelemetry`] for the stall line to read, counts every
    /// poll of the acquire future, and wraps the waker so every wake of this
    /// waiter is counted too. The registration is removed by a drop guard, so a
    /// caller cancelled mid-wait (a dropped handler future) cannot leave a
    /// ghost waiter behind for the report to accuse.
    async fn lock_contended(
        &self,
        site: &'static str,
    ) -> tokio::sync::MutexGuard<'_, HashMap<Uri, DocumentState>> {
        let now = crate::rt::Instant::now();
        let telemetry = Arc::new(std::sync::Mutex::new(WaiterTelemetry {
            site,
            parked_since: now,
            polls: 0,
            last_poll: now,
            wakes: 0,
            last_wake: None,
            #[cfg(not(target_family = "wasm"))]
            task_id: tokio::task::try_id(),
        }));
        self.tracking().waiters.push(Arc::clone(&telemetry));
        let _deregister = DeregisterWaiter {
            store: self,
            telemetry: Arc::clone(&telemetry),
        };
        InstrumentedAcquire {
            inner: Box::pin(self.docs.lock()),
            telemetry,
        }
        .await
    }

    /// Every current waiter, aged against one clock reading.
    pub(in super::super) fn waiters(&self) -> Vec<WaiterSnapshot> {
        let now = crate::rt::Instant::now();
        let entries: Vec<_> = self.tracking().waiters.clone();
        entries
            .iter()
            .map(|entry| {
                let t = entry
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                WaiterSnapshot {
                    #[cfg(not(target_family = "wasm"))]
                    telemetry_id: Arc::as_ptr(entry) as usize,
                    site: t.site,
                    parked_for: now.duration_since(t.parked_since),
                    polls: t.polls,
                    since_last_poll: now.duration_since(t.last_poll),
                    wakes: t.wakes,
                    since_last_wake: t.last_wake.map(|w| now.duration_since(w)),
                    #[cfg(not(target_family = "wasm"))]
                    task_id: t.task_id,
                }
            })
            .collect()
    }

    /// One coherent watchdog sample. Holder, acquisition counter and waiter
    /// registrations are cloned under one tracking lock and aged from one
    /// clock reading, so the detector cannot combine states that never existed
    /// together.
    #[cfg(not(target_family = "wasm"))]
    pub(in super::super) fn watchdog_snapshot(&self) -> (DocumentsContention, Vec<WaiterSnapshot>) {
        let now = crate::rt::Instant::now();
        let (held_by, last, acquisitions, waiter_entries) = {
            let tracking = self.tracking();
            (
                tracking.holder.map(|h| held_for(&h, now)),
                tracking
                    .last
                    .map(|h| (h.site, now.duration_since(h.since), h.longest_phase)),
                tracking.acquisitions,
                tracking.waiters.clone(),
            )
        };
        let contention = DocumentsContention {
            held_by,
            last,
            acquisitions,
        };
        let waiters = waiter_entries
            .iter()
            .map(|entry| {
                let t = entry
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                WaiterSnapshot {
                    telemetry_id: Arc::as_ptr(entry) as usize,
                    site: t.site,
                    parked_for: now.duration_since(t.parked_since),
                    polls: t.polls,
                    since_last_poll: now.duration_since(t.last_poll),
                    wakes: t.wakes,
                    since_last_wake: t.last_wake.map(|w| now.duration_since(w)),
                    task_id: t.task_id,
                }
            })
            .collect();
        (contention, waiters)
    }

    /// Everything the stall line needs about this map in one sample.
    ///
    /// Taken together rather than field by field so the three readings describe
    /// the same instant; sampling them separately across a stall report would
    /// let the map be taken between them and produce a line that never
    /// corresponded to any real moment.
    pub(in super::super) fn contention(&self) -> DocumentsContention {
        // One lock, one clock reading. The lock makes the three fields describe
        // the same *state*; the clock reading makes their ages describe the same
        // *instant*. Both are needed: one clock reading alone would still let
        // one task's identity be paired with another's metadata.
        let now = crate::rt::Instant::now();
        let (held_by, last, acquisitions) = {
            let tracking = self.tracking();
            (
                tracking.holder.map(|h| held_for(&h, now)),
                tracking
                    .last
                    .map(|h| (h.site, now.duration_since(h.since), h.longest_phase)),
                tracking.acquisitions,
            )
        };
        DocumentsContention {
            held_by,
            last,
            acquisitions,
        }
    }

    /// Rename whoever currently holds the map, without needing their guard.
    ///
    /// [`DocumentsGuard::retag`] is the same operation for a caller that has the
    /// guard in hand. This one exists for code that runs *under* a caller's
    /// guard without being given it — `DeliveryCtx::cache_and_enqueue`, which
    /// both of its callers invoke while holding the map, and whose pull-cache
    /// await is worth naming separately.
    ///
    /// **Precondition:** the calling task must be the holder. There is no way to
    /// check that, so a call from a task that does not hold the map would
    /// silently relabel someone else's hold. Every caller must be reachable only
    /// from under the guard.
    pub(in super::super) fn retag_held(&self, site: &'static str) {
        if let Some(holder) = self.tracking().holder.as_mut() {
            let now = crate::rt::Instant::now();
            holder.close_phase(now);
            holder.site = site;
            holder.phase_since = now;
        }
    }

    /// The tracking record, with a poisoned lock treated as merely stale.
    ///
    /// A panic while holding it leaves diagnostic bookkeeping possibly
    /// half-written, which is worth reading anyway — refusing to report because
    /// a previous panic poisoned the log's own metadata would be the wrong
    /// trade during exactly the failure this metadata exists to explain.
    pub(in super::super) fn tracking(&self) -> std::sync::MutexGuard<'_, DocumentsTracking> {
        self.tracking
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The current holder and how long it has held, for the stall line.
    ///
    /// Takes no `await` and never blocks on the map itself, so it is safe to
    /// call from a reporter that is itself running because the map is stuck.
    #[cfg(test)]
    pub(in super::super) fn holder(&self) -> Option<HeldFor> {
        self.contention().held_by
    }
}

/// Removes a [`WaiterTelemetry`] registration when the acquire ends — by
/// success **or** by cancellation. Identity is the `Arc` pointer, so two
/// waiters from the same site never remove each other's entries.
struct DeregisterWaiter<'a> {
    pub(in super::super) store: &'a DocumentStore,
    telemetry: Arc<std::sync::Mutex<WaiterTelemetry>>,
}

impl Drop for DeregisterWaiter<'_> {
    fn drop(&mut self) {
        self.store
            .tracking()
            .waiters
            .retain(|w| !Arc::ptr_eq(w, &self.telemetry));
    }
}

/// Polls the mutex acquire while recording every poll and routing the waker
/// through [`TelemetryWaker`] so every wake is recorded too.
///
/// The inner future is boxed: this only exists on the contended path, so the
/// allocation is paid exactly when a waiter exists to describe.
/// The boxed mutex-acquire future [`InstrumentedAcquire`] drives.
type BoxedMapAcquire<'a> = Pin<
    Box<dyn Future<Output = tokio::sync::MutexGuard<'a, HashMap<Uri, DocumentState>>> + Send + 'a>,
>;

struct InstrumentedAcquire<'a> {
    pub(in super::super) inner: BoxedMapAcquire<'a>,
    telemetry: Arc<std::sync::Mutex<WaiterTelemetry>>,
}

impl<'a> Future for InstrumentedAcquire<'a> {
    type Output = tokio::sync::MutexGuard<'a, HashMap<Uri, DocumentState>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        {
            let mut t = self
                .telemetry
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            t.polls += 1;
            t.last_poll = crate::rt::Instant::now();
        }
        // A fresh wrapper per poll, deliberately: futures may only use the
        // waker from their *most recent* poll, and tokio's mutex replaces its
        // registered waker on each poll — reusing one wrapper across polls
        // would record wakes delivered to a waker the mutex no longer holds.
        let waker = std::task::Waker::from(Arc::new(TelemetryWaker {
            real: cx.waker().clone(),
            telemetry: Arc::clone(&self.telemetry),
        }));
        let mut shim_cx = std::task::Context::from_waker(&waker);
        self.inner.as_mut().poll(&mut shim_cx)
    }
}

/// A held [`DocumentStore`], which clears the holder tag when dropped.
///
/// Derefs to the map, so every call site reads exactly as it did when the field
/// was a bare `Mutex<HashMap<..>>`.
pub(in super::super) struct DocumentsGuard<'a> {
    pub(in super::super) docs: tokio::sync::MutexGuard<'a, HashMap<Uri, DocumentState>>,
    pub(in super::super) store: &'a DocumentStore,
}

impl DocumentsGuard<'_> {
    /// Rename the current holder without releasing the map.
    ///
    /// A long guarded region is not one place. `publish_diagnostics_result`
    /// holds this map across six `await`s — a `workspace_index` write, a slot
    /// walk, two small mutexes, and the delivery — and a tag naming only the
    /// function tells you it was stuck somewhere in that span. Retagging as it
    /// advances narrows "held by `publish_diagnostics_result` for 103.7s" to the
    /// exact `await`, the same way [`EditTurn::at`] does for a held turn.
    ///
    /// [`DocumentsHolder::since`] is deliberately **not** reset, so the reported
    /// age stays the age of the whole hold rather than of the current step; the
    /// site says where it stopped, the age says how long the map has been gone.
    pub(in super::super) fn retag(&self, site: &'static str) {
        self.store.retag_held(site);
    }
}

impl std::ops::Deref for DocumentsGuard<'_> {
    type Target = HashMap<Uri, DocumentState>;

    fn deref(&self) -> &Self::Target {
        &self.docs
    }
}

impl std::ops::DerefMut for DocumentsGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.docs
    }
}

impl Drop for DocumentsGuard<'_> {
    fn drop(&mut self) {
        // Move the departing holder aside rather than discarding it, so a stall
        // line that finds the map free can still say who had it last and how
        // long ago they let go. `since` is left as the *hold's*
        // start, so the reported age reads as "released, having held from N
        // seconds ago" — the release instant is recoverable from the pair.
        let mut tracking = self.store.tracking();
        if let Some(mut departing) = tracking.holder.take() {
            // The phase that was running at release is a finished phase like
            // any other, and it is the one a hold whose *last* step is the slow
            // one would otherwise never record.
            departing.close_phase(crate::rt::Instant::now());
            tracking.last = Some(departing);
        }
    }
}

/// Age a [`DocumentsHolder`] against one clock reading.
///
/// Both durations come from the same `now`, so the phase age can never exceed
/// the hold age that contains it — an impossible reading two separate
/// `elapsed()` calls can produce.
fn held_for(h: &DocumentsHolder, now: crate::rt::Instant) -> HeldFor {
    HeldFor {
        site: h.site,
        held: now.duration_since(h.since),
        in_phase: now.duration_since(h.phase_since),
        longest_phase: h.longest_phase,
    }
}
