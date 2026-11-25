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

//! Locks with holder and waiter attribution for the query database and the workspace index, and the census of live database snapshots.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tower_lsp_server::ls_types::Uri;

/// [`salsa::Cancelled::catch`] for a read on a tracked database clone that
/// also stops *inside* a long query.
///
/// Query boundaries are too coarse on their own: one compilation unit or token
/// walk runs for seconds without entering another query, so a cancelled clone
/// kept the publication (and the edit behind it) waiting. The compiler's
/// cooperative checkpoints probe this clone's token and unwind with
/// `Cancelled::Local`; Salsa never memoises a query that unwound. The token is
/// read without Salsa's "local cancellation disabled" bit, which only cycle
/// recovery sets; `tcl-lsp-db` declares no cycle-recovering query.
pub(super) fn cancellable_catch<T>(
    db: &tcl_lsp_db::TclDatabase,
    read: impl FnOnce() -> T,
) -> Result<T, salsa::Cancelled> {
    use tcl_compiler::analysis_control::{AnalysisControl, run};
    let token = salsa::Database::cancellation_token(db);
    let control = AnalysisControl::new(move |_| {
        if token.is_cancelled() {
            std::panic::resume_unwind(Box::new(salsa::Cancelled::Local));
        }
        false
    });
    // Callers hand over owned clones; an unwind leaves nothing half-observed.
    salsa::Cancelled::catch(std::panic::AssertUnwindSafe(|| {
        run(control, read)
            .unwrap_or_else(|_| std::panic::resume_unwind(Box::new(salsa::Cancelled::Local)))
    }))
}

/// A tracked clone's handle on its own in-flight queries.
///
/// A live publication waits for every outstanding clone to drop. Without a
/// signal those reads ran to completion first, so an edit queued behind whole
/// stale diagnostics and token computations. Cancelling the clone's token makes
/// its queries unwind at their next query boundary (or, under
/// [`cancellable_catch`], the next compiler checkpoint), exactly as a Salsa
/// write cancels readers; every reader already treats `salsa::Cancelled` as
/// "the revision moved on".
pub(super) trait SnapshotCancellation {
    fn snapshot_cancellation(&self) -> Option<salsa::CancellationToken> {
        None
    }
}

impl SnapshotCancellation for tcl_lsp_db::TclDatabase {
    fn snapshot_cancellation(&self) -> Option<salsa::CancellationToken> {
        Some(salsa::Database::cancellation_token(self))
    }
}

impl SnapshotCancellation for HashMap<Uri, tcl_lsp_db::SourceFile> {}

pub(super) type CensusEntry = (
    &'static str,
    crate::rt::Instant,
    Option<salsa::CancellationToken>,
);

/// Live salsa database snapshots, so a stalled publication can name what it is
/// waiting for.
///
/// # Why this exists
///
/// Salsa grants `&mut` on the database through `Storage::cancel_others`, which
/// parks on a condvar until every *other* `DatabaseImpl` clone has been
/// dropped. Reaching that while holding the [`EditOrder`] turn would let one
/// slow-to-drop snapshot stop the whole server, so live publication is deferred
/// past the turn and additionally requires its database's census to be empty
/// before entering a setter.
///
/// A stall report can name the barrier that stopped and the handler holding it,
/// but not *what that handler is waiting for*, because a snapshot otherwise
/// leaves no trace — and the process is gone by the time anyone looks. This is
/// that trace: every clone registers here with the site that made it and when,
/// and retires on drop.
///
/// The census belongs to the [`TrackedMutex`] that owns one Salsa database.
/// That scope is a correctness property: a snapshot from an unrelated backend
/// must not delay this backend's publication.
///
/// # Cost
///
/// One `HashMap` insert and one remove per salsa query, under a
/// `std::sync::Mutex` whose critical sections contain no `await` and no
/// allocation beyond the entry itself. Against the query it accompanies — a
/// file analysis, a semantic-token pass, a whole-project arity walk — that is
/// not measurable. Free functions already receive the tracked mutex, so they
/// automatically register with the correct database's census.
#[derive(Default)]
pub(super) struct SnapshotCensus {
    pub(super) live: std::sync::Mutex<HashMap<u64, CensusEntry>>,
    pub(super) next_id: std::sync::atomic::AtomicU64,
    pub(super) publication_intent: std::sync::atomic::AtomicUsize,
    pub(super) publication_advanced: tokio::sync::Notify,
}

/// One publisher's ownership of the intent to drain request snapshots before
/// a Salsa source setter runs.
///
/// Snapshot creation checks the intent both before queueing for the database
/// mutex and while holding it. A publisher can therefore establish intent
/// before its first optimistic lock attempt: existing database waiters drain,
/// while new readers neither extend the snapshot census nor replenish the fair
/// mutex queue ahead of the writer. Dropping the guard releases every reader
/// queued behind the live publication.
pub(super) struct SnapshotDrainGuard {
    pub(super) census: Arc<SnapshotCensus>,
}

impl Drop for SnapshotDrainGuard {
    fn drop(&mut self) {
        let previous = self
            .census
            .publication_intent
            .fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
        debug_assert!(previous > 0, "snapshot-drain ownership must be balanced");
        if previous == 1 {
            self.census.publication_advanced.notify_waiters();
        }
    }
}

/// What the census can say about the snapshots outstanding right now.
pub(super) struct SnapshotReport {
    pub(super) outstanding: usize,
    /// The site and age of the longest-lived one — the one a blocked
    /// `cancel_others` is really waiting for.
    pub(super) oldest: Option<(&'static str, std::time::Duration)>,
}

impl SnapshotCensus {
    pub(super) fn begin_publication(self: &Arc<Self>) -> SnapshotDrainGuard {
        self.publication_intent
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        // New readers are now refused; the outstanding ones read a revision
        // this publication supersedes, so stop them rather than wait them out.
        for (_, _, token) in self
            .live
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
        {
            if let Some(token) = token {
                token.cancel();
            }
        }
        SnapshotDrainGuard {
            census: Arc::clone(self),
        }
    }

    pub(super) fn try_track<T: SnapshotCancellation>(
        self: &Arc<Self>,
        value: T,
        site: &'static str,
    ) -> Result<DbSnapshot<T>, T> {
        if self
            .publication_intent
            .load(std::sync::atomic::Ordering::Acquire)
            != 0
        {
            return Err(value);
        }
        Ok(self.track(value, site))
    }

    /// Put an owned value behind the same census lifetime as a database clone.
    ///
    /// Generic only so the destructor ordering can be proved with a drop probe;
    /// production calls this through [`TrackedMutex::snapshot`] with
    /// `TclDatabase`.
    pub(super) fn track<T: SnapshotCancellation>(
        self: &Arc<Self>,
        value: T,
        site: &'static str,
    ) -> DbSnapshot<T> {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.live
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
                id,
                (
                    site,
                    crate::rt::Instant::now(),
                    value.snapshot_cancellation(),
                ),
            );
        DbSnapshot {
            db: Some(value),
            census: Arc::clone(self),
            id,
        }
    }

    pub(super) fn retire(&self, id: u64) {
        self.live
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&id);
    }

    pub(super) fn report(&self) -> SnapshotReport {
        let live = self
            .live
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Ordered by *age*, not by the instants themselves. The wasm arm of
        // `crate::rt::Instant` is `f64` milliseconds from `performance.now()`,
        // so it is `PartialOrd` and deliberately not `Ord` — floats have no
        // total order, and manufacturing one for a clock would be a lie about
        // NaN rather than a convenience. `Duration` is `Ord` on both targets.
        //
        // It also reads each clock exactly once, so the age this reports is the
        // same number the comparison was decided on.
        let oldest = live
            .values()
            .map(|(site, since, _)| (*site, since.elapsed()))
            .max_by_key(|(_, age)| *age);
        SnapshotReport {
            outstanding: live.len(),
            oldest,
        }
    }
}

/// A salsa database snapshot that retires itself from its database's census.
///
/// Derefs to the database, so a call site reads `&*snapshot` rather than
/// `&snapshot` and is otherwise unchanged. Move it into the worker it was
/// cloned for and let it drop there — see [`Backend::db_set_source`] for what
/// holding one across an unrelated `await` costs.
pub(super) struct DbSnapshot<T = tcl_lsp_db::TclDatabase> {
    // `Option` lets `Drop` destroy the value explicitly before retiring its
    // census entry. Rust otherwise runs `Drop::drop` before dropping fields,
    // which would leave a false-zero window while the Salsa clone still lives.
    pub(super) db: Option<T>,
    pub(super) census: Arc<SnapshotCensus>,
    pub(super) id: u64,
}

impl<T> std::ops::Deref for DbSnapshot<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.db
            .as_ref()
            .expect("a tracked snapshot is never accessed while dropping")
    }
}

impl<T> Drop for DbSnapshot<T> {
    fn drop(&mut self) {
        // The census is also a publication-safety barrier, not merely
        // telemetry: zero means a Salsa setter cannot enter `cancel_others`.
        // Destroy the clone synchronously before making that assertion true.
        drop(self.db.take());
        self.census.retire(self.id);
    }
}

/// A Tokio mutex with low-cost holder/waiter telemetry for the two Salsa
/// stores on the live-publication path.
///
/// Unlike the document store's task-poll instrumentation below, this records
/// call sites and ages only. That is what a stall report needs: when a live
/// publication cannot acquire `db` or `db_files`, the stall line can name the
/// current (or last) holder and the oldest queued waiter instead of merely
/// reporting that `try_lock` failed.
pub(super) struct TrackedMutex<T> {
    pub(super) value: Mutex<T>,
    pub(super) name: &'static str,
    pub(super) tracking: std::sync::Mutex<TrackedMutexState>,
    pub(super) snapshots: Arc<SnapshotCensus>,
}

#[derive(Default)]
pub(super) struct TrackedMutexState {
    pub(super) next_id: u64,
    pub(super) holder: Option<TrackedMutexActor>,
    last_holder: Option<TrackedMutexActor>,
    pub(super) waiters: HashMap<u64, TrackedMutexActor>,
}

#[derive(Clone, Copy)]
pub(super) struct TrackedMutexActor {
    pub(super) id: u64,
    pub(super) site: &'static std::panic::Location<'static>,
    pub(super) since: crate::rt::Instant,
}

struct TrackedMutexWaiter<'a> {
    pub(super) tracking: &'a std::sync::Mutex<TrackedMutexState>,
    pub(super) id: u64,
}

impl Drop for TrackedMutexWaiter<'_> {
    fn drop(&mut self) {
        self.tracking
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .waiters
            .remove(&self.id);
    }
}

pub(super) struct TrackedMutexGuard<'a, T> {
    pub(super) value: tokio::sync::MutexGuard<'a, T>,
    pub(super) tracking: &'a std::sync::Mutex<TrackedMutexState>,
    holder_id: u64,
}

impl<T> std::ops::Deref for TrackedMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> std::ops::DerefMut for TrackedMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T> Drop for TrackedMutexGuard<'_, T> {
    fn drop(&mut self) {
        let mut tracking = self
            .tracking
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if tracking
            .holder
            .is_some_and(|holder| holder.id == self.holder_id)
        {
            tracking.last_holder = tracking.holder.take();
        }
    }
}

impl<T> TrackedMutex<T> {
    pub(super) fn new(name: &'static str, value: T) -> Self {
        Self {
            value: Mutex::new(value),
            name,
            tracking: std::sync::Mutex::new(TrackedMutexState::default()),
            snapshots: Arc::new(SnapshotCensus::default()),
        }
    }

    /// Clone the protected value and register the clone before releasing the
    /// mutex. A setter holding this mutex can therefore trust a zero census:
    /// no new snapshot can appear until that setter releases it.
    pub(super) async fn snapshot(&self, site: &'static str) -> DbSnapshot<T>
    where
        T: Clone + SnapshotCancellation,
    {
        loop {
            let publication_advanced = self.snapshots.publication_advanced.notified();
            tokio::pin!(publication_advanced);
            publication_advanced.as_mut().enable();
            if self
                .snapshots
                .publication_intent
                .load(std::sync::atomic::Ordering::Acquire)
                != 0
            {
                publication_advanced.await;
                continue;
            }
            let snapshot = {
                let value = self.lock().await;
                self.snapshots.try_track(value.clone(), site)
            };
            if let Ok(snapshot) = snapshot {
                return snapshot;
            }
            publication_advanced.await;
        }
    }

    /// Register a clone while the caller already holds this mutex, or decline
    /// it when a live publisher is draining the finite pre-existing set.
    pub(super) fn snapshot_from(&self, value: &T, site: &'static str) -> Option<DbSnapshot<T>>
    where
        T: Clone + SnapshotCancellation,
    {
        self.snapshots.try_track(value.clone(), site).ok()
    }

    pub(super) fn begin_snapshot_drain(&self) -> SnapshotDrainGuard {
        self.snapshots.begin_publication()
    }

    pub(super) fn snapshot_report(&self) -> SnapshotReport {
        self.snapshots.report()
    }

    #[track_caller]
    pub(super) fn lock(&self) -> impl Future<Output = TrackedMutexGuard<'_, T>> {
        let site = std::panic::Location::caller();
        async move {
            if let Ok(value) = self.value.try_lock() {
                return self.record_holder(value, site);
            }
            let waiter = {
                let mut tracking = self
                    .tracking
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                let id = tracking.next_id;
                tracking.next_id = tracking.next_id.wrapping_add(1);
                tracking.waiters.insert(
                    id,
                    TrackedMutexActor {
                        id,
                        site,
                        since: crate::rt::Instant::now(),
                    },
                );
                TrackedMutexWaiter {
                    tracking: &self.tracking,
                    id,
                }
            };
            let value = self.value.lock().await;
            drop(waiter);
            self.record_holder(value, site)
        }
    }

    #[track_caller]
    pub(super) fn try_lock(&self) -> Result<TrackedMutexGuard<'_, T>, tokio::sync::TryLockError> {
        let site = std::panic::Location::caller();
        self.value
            .try_lock()
            .map(|value| self.record_holder(value, site))
    }

    fn record_holder<'a>(
        &'a self,
        value: tokio::sync::MutexGuard<'a, T>,
        site: &'static std::panic::Location<'static>,
    ) -> TrackedMutexGuard<'a, T> {
        let mut tracking = self
            .tracking
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let id = tracking.next_id;
        tracking.next_id = tracking.next_id.wrapping_add(1);
        tracking.holder = Some(TrackedMutexActor {
            id,
            site,
            since: crate::rt::Instant::now(),
        });
        TrackedMutexGuard {
            value,
            tracking: &self.tracking,
            holder_id: id,
        }
    }

    pub(super) fn contention(&self) -> String {
        let tracking = self
            .tracking
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let holder = tracking.holder.or(tracking.last_holder);
        let holder_kind = if tracking.holder.is_some() {
            "held"
        } else {
            "free; last held"
        };
        let holder = holder.map_or_else(
            || "never acquired".to_owned(),
            |actor| {
                format!(
                    "{holder_kind} by {} for {:.1}s",
                    actor.site,
                    actor.since.elapsed().as_secs_f64(),
                )
            },
        );
        // The browser runtime uses an f64-backed monotonic clock, so its
        // Instant is only partially ordered. Worker timestamps are finite in
        // practice; keep telemetry robust if a host ever supplies NaN.
        let oldest_waiter = tracking.waiters.values().min_by(|left, right| {
            left.since
                .partial_cmp(&right.since)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let waiters = oldest_waiter.map_or_else(
            || "no queued waiters".to_owned(),
            |actor| {
                format!(
                    "{} queued waiter(s); oldest at {} for {:.1}s",
                    tracking.waiters.len(),
                    actor.site,
                    actor.since.elapsed().as_secs_f64(),
                )
            },
        );
        format!("{}: {holder}; {waiters}", self.name)
    }
}

/// A Tokio `RwLock` with holder/waiter attribution, for the workspace index.
///
/// The stall line names who holds the document map ([`DocumentStore`]) and
/// the Salsa stores ([`TrackedMutex`]); this lets it name who holds the index
/// as well, rather than only who is waiting for it.
///
/// A read/write lock has more than one holder at a time, so this keeps every
/// live reader alongside the single writer, the last of each to release, and
/// every queued acquire with the access it wants. A holder's identity is the
/// call site (`#[track_caller]`, as [`TrackedMutex`] does) plus a phase label
/// it can set through [`TrackedReadGuard::retag`] / [`TrackedWriteGuard::retag`],
/// the same phase clock [`DocumentsHolder`] carries.
pub(super) struct TrackedRwLock<T> {
    pub(super) value: RwLock<T>,
    pub(super) name: &'static str,
    pub(super) tracking: std::sync::Mutex<TrackedRwLockState>,
}

#[derive(Default)]
pub(super) struct TrackedRwLockState {
    pub(super) next_id: u64,
    pub(super) writer: Option<RwLockActor>,
    readers: HashMap<u64, RwLockActor>,
    last_writer: Option<ReleasedRwLockActor>,
    last_reader: Option<ReleasedRwLockActor>,
    pub(super) waiters: HashMap<u64, (RwLockAccess, RwLockActor)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RwLockAccess {
    Read,
    Write,
}

impl RwLockAccess {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
        }
    }
}

/// One live holder or queued waiter of a [`TrackedRwLock`].
#[derive(Clone, Copy)]
pub(super) struct RwLockActor {
    pub(super) id: u64,
    pub(super) site: &'static std::panic::Location<'static>,
    /// The step the holder is at, set by `retag`; `None` until the first one.
    pub(super) phase: Option<&'static str>,
    /// When the guard was acquired (or the waiter queued). Never reset by a
    /// retag, so the age is the whole hold's.
    pub(super) since: crate::rt::Instant,
    /// When `phase` was last set — the age of the current step.
    pub(super) phase_since: crate::rt::Instant,
}

impl RwLockActor {
    pub(super) fn new(id: u64, site: &'static std::panic::Location<'static>) -> Self {
        let now = crate::rt::Instant::now();
        Self {
            id,
            site,
            phase: None,
            since: now,
            phase_since: now,
        }
    }

    /// `site` plus the phase label, when one has been set.
    pub(super) fn describe(&self) -> String {
        match self.phase {
            Some(phase) => format!("{} `{phase}`", self.site),
            None => self.site.to_string(),
        }
    }
}

/// A released holder, kept so a free lock can still say who had it last.
#[derive(Clone, Copy)]
pub(super) struct ReleasedRwLockActor {
    actor: RwLockActor,
    pub(super) released: crate::rt::Instant,
}

struct TrackedRwLockWaiter<'a> {
    pub(super) tracking: &'a std::sync::Mutex<TrackedRwLockState>,
    pub(super) id: u64,
}

impl Drop for TrackedRwLockWaiter<'_> {
    fn drop(&mut self) {
        self.tracking
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .waiters
            .remove(&self.id);
    }
}

pub(super) struct TrackedReadGuard<'a, T> {
    pub(super) value: tokio::sync::RwLockReadGuard<'a, T>,
    pub(super) tracking: &'a std::sync::Mutex<TrackedRwLockState>,
    pub(super) id: u64,
}

#[cfg(test)]
impl<T> TrackedReadGuard<'_, T> {
    /// Name the step this reader is at, without releasing it.
    pub(super) fn retag(&self, phase: &'static str) {
        let mut tracking = self
            .tracking
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(actor) = tracking.readers.get_mut(&self.id) {
            actor.phase = Some(phase);
            actor.phase_since = crate::rt::Instant::now();
        }
    }
}

impl<T> std::ops::Deref for TrackedReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> Drop for TrackedReadGuard<'_, T> {
    fn drop(&mut self) {
        let mut tracking = self
            .tracking
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(actor) = tracking.readers.remove(&self.id) {
            tracking.last_reader = Some(ReleasedRwLockActor {
                actor,
                released: crate::rt::Instant::now(),
            });
        }
    }
}

pub(super) struct TrackedWriteGuard<'a, T> {
    pub(super) value: tokio::sync::RwLockWriteGuard<'a, T>,
    pub(super) tracking: &'a std::sync::Mutex<TrackedRwLockState>,
    pub(super) id: u64,
}

impl<T> TrackedWriteGuard<'_, T> {
    /// Name the step this writer is at, without releasing it.
    ///
    /// The hold's `since` is deliberately left alone, exactly as
    /// [`DocumentsGuard::retag`] leaves the map's: the report's age is the age
    /// of the whole hold, and the phase clock says how long *this* step has
    /// taken.
    pub(super) fn retag(&self, phase: &'static str) {
        let mut tracking = self
            .tracking
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(actor) = tracking.writer.as_mut().filter(|actor| actor.id == self.id) {
            actor.phase = Some(phase);
            actor.phase_since = crate::rt::Instant::now();
        }
    }
}

impl<T> std::ops::Deref for TrackedWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> std::ops::DerefMut for TrackedWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T> Drop for TrackedWriteGuard<'_, T> {
    fn drop(&mut self) {
        let mut tracking = self
            .tracking
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if tracking.writer.is_some_and(|actor| actor.id == self.id) {
            tracking.last_writer = tracking.writer.take().map(|actor| ReleasedRwLockActor {
                actor,
                released: crate::rt::Instant::now(),
            });
        }
    }
}

impl<T> TrackedRwLock<T> {
    pub(super) fn new(name: &'static str, value: T) -> Self {
        Self {
            value: RwLock::new(value),
            name,
            tracking: std::sync::Mutex::new(TrackedRwLockState::default()),
        }
    }

    pub(super) fn tracking(&self) -> std::sync::MutexGuard<'_, TrackedRwLockState> {
        self.tracking
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(super) fn next_id(tracking: &mut TrackedRwLockState) -> u64 {
        let id = tracking.next_id;
        tracking.next_id = tracking.next_id.wrapping_add(1);
        id
    }

    fn register_waiter(
        &self,
        access: RwLockAccess,
        site: &'static std::panic::Location<'static>,
    ) -> TrackedRwLockWaiter<'_> {
        let mut tracking = self.tracking();
        let id = Self::next_id(&mut tracking);
        tracking
            .waiters
            .insert(id, (access, RwLockActor::new(id, site)));
        TrackedRwLockWaiter {
            tracking: &self.tracking,
            id,
        }
    }

    fn record_reader<'a>(
        &'a self,
        value: tokio::sync::RwLockReadGuard<'a, T>,
        site: &'static std::panic::Location<'static>,
    ) -> TrackedReadGuard<'a, T> {
        let mut tracking = self.tracking();
        let id = Self::next_id(&mut tracking);
        tracking.readers.insert(id, RwLockActor::new(id, site));
        TrackedReadGuard {
            value,
            tracking: &self.tracking,
            id,
        }
    }

    fn record_writer<'a>(
        &'a self,
        value: tokio::sync::RwLockWriteGuard<'a, T>,
        site: &'static std::panic::Location<'static>,
    ) -> TrackedWriteGuard<'a, T> {
        let mut tracking = self.tracking();
        let id = Self::next_id(&mut tracking);
        tracking.writer = Some(RwLockActor::new(id, site));
        TrackedWriteGuard {
            value,
            tracking: &self.tracking,
            id,
        }
    }

    /// Shared access, queued fairly behind any pending writer exactly as the
    /// bare lock's `read` is. The uncontended path records nothing but the
    /// holder; a waiter is registered only when the acquire has to park.
    #[track_caller]
    pub(super) fn read(&self) -> impl Future<Output = TrackedReadGuard<'_, T>> {
        let site = std::panic::Location::caller();
        async move {
            if let Ok(value) = self.value.try_read() {
                return self.record_reader(value, site);
            }
            let waiter = self.register_waiter(RwLockAccess::Read, site);
            let value = self.value.read().await;
            drop(waiter);
            self.record_reader(value, site)
        }
    }

    /// Exclusive access, with the same fair queue as the bare lock's `write`.
    #[track_caller]
    pub(super) fn write(&self) -> impl Future<Output = TrackedWriteGuard<'_, T>> {
        let site = std::panic::Location::caller();
        async move {
            if let Ok(value) = self.value.try_write() {
                return self.record_writer(value, site);
            }
            let waiter = self.register_waiter(RwLockAccess::Write, site);
            let value = self.value.write().await;
            drop(waiter);
            self.record_writer(value, site)
        }
    }

    /// Nonblocking shared access with the same holder attribution as `read`.
    #[track_caller]
    pub(super) fn try_read(&self) -> Result<TrackedReadGuard<'_, T>, tokio::sync::TryLockError> {
        let site = std::panic::Location::caller();
        self.value
            .try_read()
            .map(|value| self.record_reader(value, site))
    }

    /// Shared access from a blocking worker thread, never from a task.
    #[track_caller]
    pub(super) fn blocking_read(&self) -> TrackedReadGuard<'_, T> {
        let site = std::panic::Location::caller();
        if let Ok(value) = self.value.try_read() {
            return self.record_reader(value, site);
        }
        let waiter = self.register_waiter(RwLockAccess::Read, site);
        let value = self.value.blocking_read();
        drop(waiter);
        self.record_reader(value, site)
    }

    #[track_caller]
    pub(super) fn try_write(&self) -> Result<TrackedWriteGuard<'_, T>, tokio::sync::TryLockError> {
        let site = std::panic::Location::caller();
        self.value
            .try_write()
            .map(|value| self.record_writer(value, site))
    }

    /// The stall line's clause for this lock: who holds it (or held it last),
    /// at which step, for how long, and who is queued behind them.
    pub(super) fn contention(&self) -> String {
        let now = crate::rt::Instant::now();
        let tracking = self.tracking();
        let age = |since: crate::rt::Instant| now.duration_since(since).as_secs_f64();
        let holder = if let Some(writer) = tracking.writer {
            format!(
                "held for write by {} — {:.1}s in total, {:.1}s at this point",
                writer.describe(),
                age(writer.since),
                age(writer.phase_since),
            )
        } else if tracking.readers.is_empty() {
            let released = |kind: &str, last: Option<ReleasedRwLockActor>| {
                last.map_or_else(
                    || format!("never {kind}"),
                    |last| {
                        format!(
                            "last {kind} by {}, released {:.1}s ago after {:.1}s",
                            last.actor.describe(),
                            age(last.released),
                            last.released.duration_since(last.actor.since).as_secs_f64(),
                        )
                    },
                )
            };
            format!(
                "free; {}; {}",
                released("written", tracking.last_writer),
                released("read", tracking.last_reader),
            )
        } else {
            // Oldest first: the reader a queued writer has been waiting on the
            // longest is the one to look at.
            let mut readers: Vec<&RwLockActor> = tracking.readers.values().collect();
            readers.sort_by(|a, b| {
                a.since
                    .partial_cmp(&b.since)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let listed: Vec<String> = readers
                .iter()
                .take(4)
                .map(|reader| {
                    format!(
                        "{} ({:.1}s, {:.1}s at this point)",
                        reader.describe(),
                        age(reader.since),
                        age(reader.phase_since),
                    )
                })
                .collect();
            let elided = readers.len().saturating_sub(listed.len());
            let more = if elided == 0 {
                String::new()
            } else {
                format!(" and {elided} more")
            };
            format!(
                "held for read by {} reader(s): {}{more}",
                readers.len(),
                listed.join(", "),
            )
        };
        let queue = |access: RwLockAccess| {
            let oldest = tracking
                .waiters
                .values()
                .filter(|(kind, _)| *kind == access)
                .map(|(_, actor)| actor)
                .min_by(|a, b| {
                    a.since
                        .partial_cmp(&b.since)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            let count = tracking
                .waiters
                .values()
                .filter(|(kind, _)| *kind == access)
                .count();
            oldest.map(|actor| {
                format!(
                    "{count} queued {}er(s), oldest {} for {:.1}s",
                    access.label(),
                    actor.site,
                    age(actor.since),
                )
            })
        };
        let waiters = match (queue(RwLockAccess::Write), queue(RwLockAccess::Read)) {
            (None, None) => "no queued waiters".to_owned(),
            (Some(writers), None) => writers,
            (None, Some(readers)) => readers,
            (Some(writers), Some(readers)) => format!("{writers}; {readers}"),
        };
        format!("{}: {holder}; {waiters}", self.name)
    }
}
