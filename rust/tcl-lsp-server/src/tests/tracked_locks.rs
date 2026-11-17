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

//! Tracked lock contention reports and the snapshot census.

use super::*;

/// A stall report must identify both the Salsa store owner and its queued
/// demand, not stop at "`try_lock` failed".
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn salsa_store_contention_names_holder_and_waiter_1849() {
    let store = Arc::new(TrackedMutex::new("db", ()));
    let held = store.lock().await;
    let waiting = crate::rt::spawn({
        let store = Arc::clone(&store);
        async move { drop(store.lock().await) }
    });

    let report = crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let report = store.contention();
            if report.contains("1 queued waiter(s)") {
                break report;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the tracked waiter must become visible");
    assert!(report.contains("db: held by"), "holder missing: {report}");
    assert!(
        report.contains("oldest at"),
        "waiter site missing: {report}"
    );

    drop(held);
    waiting.await.expect("tracked waiter must finish");
}

#[test]
fn a_cancelled_clone_unwinds_at_the_next_compiler_checkpoint() {
    use tcl_compiler::analysis_control::{AnalysisCheckpoint, checkpoint};
    let db = tcl_lsp_db::TclDatabase::default();
    let reached = std::sync::atomic::AtomicBool::new(false);
    assert_eq!(
        cancellable_catch(&db, || !checkpoint(AnalysisCheckpoint::Command)).ok(),
        Some(true),
        "an uncancelled read completes and is not interrupted"
    );
    salsa::Database::cancellation_token(&db).cancel();
    let read = cancellable_catch(&db, || {
        let _ = checkpoint(AnalysisCheckpoint::Command);
        reached.store(true, std::sync::atomic::Ordering::Relaxed);
    });
    assert!(matches!(read, Err(salsa::Cancelled::Local)));
    assert!(
        !reached.load(std::sync::atomic::Ordering::Relaxed),
        "work after the checkpoint never runs"
    );
}

#[test]
fn a_live_publication_cancels_outstanding_database_clones() {
    let census = Arc::new(SnapshotCensus::default());
    let db = tcl_lsp_db::TclDatabase::default();
    let snapshot = census.track(db.clone(), "test-stale-reader");
    let token = salsa::Database::cancellation_token(&*snapshot);
    assert!(!token.is_cancelled(), "a fresh reader runs uncancelled");
    let drain = census.begin_publication();
    assert!(
        token.is_cancelled(),
        "a publication must stop the stale reader instead of waiting it out"
    );
    assert!(
        census.try_track(db.clone(), "test-late-reader").is_err(),
        "no new reader is admitted while the publication drains"
    );
    drop(snapshot);
    drop(drain);
    let late = census.track(db, "test-next-reader");
    assert!(
        !salsa::Database::cancellation_token(&*late).is_cancelled(),
        "a reader admitted after the publication is not cancelled"
    );
}

/// The snapshot census must count what is live and retire what is not.
///
/// This is the reading that names what a stalled turn is waiting for: salsa
/// parks `set_text` in `cancel_others` until every other database clone
/// drops, so the oldest outstanding snapshot is the thing at the bottom of
/// the stack. A census that over-counts would accuse an innocent site; one
/// that under-counts would report "nothing outstanding" and send the next
/// reader looking somewhere else entirely.
///
/// Driven against an isolated census so its counts are exact.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_snapshot_census_tracks_live_clones_and_retires_dropped_ones() {
    let db = tcl_lsp_db::TclDatabase::default();
    let census = Arc::new(SnapshotCensus::default());
    assert_eq!(census.report().outstanding, 0, "a fresh census is empty");

    let first = census.track(db.clone(), "test-first");
    // Distinguishable ages, so "oldest" means something to assert on.
    crate::rt::sleep(std::time::Duration::from_millis(20)).await;
    let second = census.track(db.clone(), "test-second");

    let report = census.report();
    assert_eq!(report.outstanding, 2, "both live snapshots must be counted");
    assert_eq!(
        report
            .oldest
            .expect("a live snapshot must have an oldest")
            .0,
        "test-first",
        "the census must name the *longest-lived* snapshot, not the newest — \
             that is the one a blocked `cancel_others` is waiting for",
    );

    drop(first);
    let report = census.report();
    assert_eq!(
        report.outstanding, 1,
        "a dropped snapshot must retire, or a wedge would be blamed on a query \
             that finished long ago",
    );
    assert_eq!(
        report.oldest.expect("one snapshot is still live").0,
        "test-second",
        "retiring the oldest must promote the next one",
    );

    drop(second);
    let report = census.report();
    assert_eq!(report.outstanding, 0, "the census must return to empty");
    assert!(
        report.oldest.is_none(),
        "an empty census reports no oldest, which is what tells the stall line \
             the holder is parked on something other than `cancel_others`",
    );

    // And the handle really is the database: the deref is what lets every
    // call site pass `&*snapshot` where it passed `&snapshot` before.
    let third = census.track(db.clone(), "test-deref");
    let _: &tcl_lsp_db::TclDatabase = &third;
    drop(third);
}

/// A zero census is a publication precondition, so it must
/// mean more than "the snapshot wrapper started dropping": the wrapped
/// Salsa clone must already be destroyed. Drive the exact generic drop
/// implementation with a probe whose destructor samples the census.
#[test]
fn snapshot_census_retires_only_after_the_wrapped_clone_drops_1800() {
    struct DropProbe {
        census: Arc<SnapshotCensus>,
        saw_itself_registered: Arc<std::sync::atomic::AtomicBool>,
    }
    impl SnapshotCancellation for DropProbe {}

    impl Drop for DropProbe {
        fn drop(&mut self) {
            self.saw_itself_registered.store(
                self.census.report().outstanding == 1,
                std::sync::atomic::Ordering::SeqCst,
            );
        }
    }

    let census = Arc::new(SnapshotCensus::default());
    let saw_itself_registered = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let snapshot = census.track(
        DropProbe {
            census: Arc::clone(&census),
            saw_itself_registered: Arc::clone(&saw_itself_registered),
        },
        "drop-order-proof-1800",
    );

    assert_eq!(census.report().outstanding, 1);
    drop(snapshot);
    assert!(
        saw_itself_registered.load(std::sync::atomic::Ordering::SeqCst),
        "the wrapped clone's destructor must still see its census entry"
    );
    assert_eq!(
        census.report().outstanding,
        0,
        "the entry must retire immediately after the wrapped clone is gone"
    );
}

/// The stall line's workspace-index clause names the holder, its phase,
/// and its queue.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_workspace_index_clause_names_holders_phases_and_waiters() {
    let lock = Arc::new(TrackedRwLock::new("workspace_index", 0_u32));
    assert_eq!(
        lock.contention(),
        "workspace_index: free; never written; never read; no queued waiters",
    );

    let writer = lock.write().await;
    writer.retag("publish_diagnostics_result: replace_document");
    let clause = lock.contention();
    assert!(
        clause.contains("held for write by ")
            && clause.contains("`publish_diagnostics_result: replace_document`")
            && clause.contains("tracked_locks.rs:"),
        "a writer is named with the call site that took the lock, and its phase: {clause}",
    );

    let (queued_tx, queued_rx) = tokio::sync::oneshot::channel();
    let reader = crate::rt::spawn({
        let lock = Arc::clone(&lock);
        async move {
            let _ = queued_tx.send(());
            let guard = lock.read().await;
            guard.retag("test reader: holding");
            crate::rt::sleep(std::time::Duration::from_millis(200)).await;
            drop(guard);
        }
    });
    queued_rx.await.expect("the reader task runs");
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        while !lock.contention().contains("1 queued reader(s)") {
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("a parked reader must appear in the queue");

    drop(writer);
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        while !lock.contention().contains("`test reader: holding`") {
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the reader must be recorded, with its phase, once it holds the lock");
    let clause = lock.contention();
    assert!(
        clause.contains("held for read by 1 reader(s)") && clause.contains("no queued waiters"),
        "a reader is counted and the queue is empty again: {clause}",
    );

    reader.await.expect("the reader task must not panic");
    let clause = lock.contention();
    assert!(
        clause.contains("free; last written by ")
            && clause.contains("released ")
            && clause.contains("last read by "),
        "a free lock still names its last writer and reader: {clause}",
    );
}
