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

//! The stall line and the barrier stall reporter.

use super::*;

/// The stall reporter must not announce a wedge that is already over.
///
/// `edits_settled`'s timeout can win its race by a hair: the wait expires,
/// and the edit it was waiting for lands before the report runs. Reporting
/// on the strength of "the timeout fired" would then describe a permanent
/// wedge with no holder and zero notifications queued — which is the
/// *ticket-loss* shape, so the line would not merely be noise but point at
/// the wrong defect.
///
/// The second half matters as much: recording that position in the rate
/// limiter would make a later, genuine stall at the same position suppress
/// itself. A false report could then hide the real one, in the single
/// diagnosis stream a wedge leaves behind.
///
/// Constructed directly rather than by racing a ten-second timeout: the
/// state that race produces is simply "the reporter is called with a target
/// the barrier has already reached", which is what this hands it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_barrier_that_has_caught_up_is_not_reported_as_a_stall() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///w/caught-up.tcl").unwrap();

    // One edit drawn and handed on, so the barrier has reached its target —
    // exactly the state the timeout can lose its race against.
    let target = {
        let ticket = backend.edit_order.take_ticket("didChange", &uri);
        let target = backend.edit_order.settle_target();
        drop(backend.edit_order.wait_turn(ticket).await);
        target
    };
    assert!(
        backend.edit_order.served() >= target,
        "the barrier must have caught up, or this test proves nothing",
    );

    crate::rt::timeout(
        std::time::Duration::from_secs(5),
        backend.report_edit_barrier_stall(target),
    )
    .await
    .expect("the reporter must return promptly rather than block");

    assert_eq!(
        backend
            .edit_barrier_stall_reported
            .load(std::sync::atomic::Ordering::Acquire),
        u64::MAX,
        "a barrier that has caught up must not be reported, and must not \
             claim the rate limiter's slot — doing so would suppress the next \
             genuine stall at this position (#1657)",
    );

    // The positive case still works: a turn that is genuinely held is
    // reported, and only then does the position get recorded.
    let held = backend.edit_order.take_ticket("didOpen", &uri);
    let held_at = backend.edit_order.served();
    let _turn = backend.edit_order.wait_turn(held).await;
    let queued = backend.edit_order.take_ticket("didChange", &uri);
    let stalled_target = backend.edit_order.settle_target();
    crate::rt::timeout(
        std::time::Duration::from_secs(5),
        backend.report_edit_barrier_stall(stalled_target),
    )
    .await
    .expect("the reporter must return promptly rather than block");
    assert_eq!(
        backend
            .edit_barrier_stall_reported
            .load(std::sync::atomic::Ordering::Acquire),
        held_at,
        "a genuine stall must still be reported and recorded",
    );
    drop(queued);
}

/// A stalled turn must name the `await` it is suspended at, not merely the
/// handler that holds it.
///
/// Naming the handler narrows a wedge from "the server is wedged" to
/// "`did_open` is wedged" and no further: a backtrace of the wedged
/// process shows every Tokio worker parked with an empty run queue, so the
/// holder is a suspended future that appears in no stack. The phase marker
/// is the only reading that can close that gap, and this test is what says
/// it is telling the truth rather than reporting a stale or default value.
///
/// Constructed, not raced: the test itself holds the `documents` lock that
/// `did_open` takes for its short ordered stage, so the handler is granted
/// the turn, suspends at exactly one known point, and stays there for as
/// long as the assertion needs. Nothing but this test can release it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stalled_turn_names_the_await_it_is_suspended_at() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///w/phase.tcl").unwrap();

    let held_documents = backend.documents.lock("test").await;

    let opener = {
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        tokio::spawn(async move {
            Box::pin(backend.did_open(DidOpenTextDocumentParams {
                text_document: tower_lsp_server::ls_types::TextDocumentItem {
                    uri,
                    language_id: "tcl".to_owned(),
                    version: 1,
                    text: "set x 1\n".to_owned(),
                },
            }))
            .await;
        })
    };

    let holder_now = || {
        backend
            .edit_order
            .holder
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    };
    let reached = crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if let Some(h) = holder_now()
                && h.phase == "did_open: documents.lock"
            {
                return h;
            }
            crate::rt::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await;
    let Ok(holder) = reached else {
        panic!(
            "the turn holder never reported the `await` it is suspended at. `did_open` is \
                 parked on the `documents` lock this test holds and cannot be anywhere else, so \
                 the marker is missing or stale. Without it a #1657 stall line names only the \
                 handler, which is where the investigation already was. Last observed holder: \
                 {:?}",
            holder_now(),
        )
    };

    assert_eq!(
        holder.what, "didOpen",
        "the phase must be attributed to the handler that actually holds the turn",
    );
    assert_eq!(holder.uri, uri.as_str(), "…and to the document it is for");
    assert!(
        holder.phase != TURN_PHASE_GRANTED,
        "a holder past its first marked await must not still read as freshly granted",
    );
    // The phase was entered after the grant, so a reader can tell "stuck at
    // its first await" from "got several awaits in and then stopped".
    assert!(
        holder.since.elapsed() >= holder.phase_since.elapsed(),
        "the phase cannot predate the turn that owns it",
    );

    // Release it and let the handler finish, so the test leaves no wedged
    // task behind — and so this really was a suspension, not a deadlock.
    drop(held_documents);
    crate::rt::timeout(std::time::Duration::from_secs(5), opener)
        .await
        .expect("did_open must complete once the lock is released")
        .expect("the did_open task must not panic");
    assert!(
        holder_now().is_none(),
        "the turn must be handed on, clearing the holder with it",
    );
}

/// The stall line reports all three readings, and says nothing about a
/// high-water mark it does not have.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_stall_line_names_the_longest_phase() {
    let store = DocumentStore::default();
    let before = store.contention();
    let held = store.lock("first").await;

    let fresh = describe_documents_contention(&before, &store.contention());
    assert!(fresh.contains("held by first"), "{fresh}");
    assert!(
        !fresh.contains("longest phase"),
        "a hold still in its first phase has no earlier phase to name: {fresh}",
    );

    crate::rt::sleep(std::time::Duration::from_millis(120)).await;
    held.retag("second");
    let line = describe_documents_contention(&before, &store.contention());
    assert!(line.contains("held by second"), "{line}");
    assert!(
        line.contains("longest phase so far first"),
        "the line must name the phase that burned the time: {line}",
    );
}

/// The mark covers finished phases only, so it must not be called the
/// longest while the phase running now has already outlasted it.
///
/// A hold 70s into `publish send` after a 1s earlier phase would otherwise
/// read `70.0s at this point; longest phase so far <earlier>
/// (1.0s)` — false, and pointing an investigation at the wrong step. The
/// live reading already names the longest phase in that case.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_line_does_not_call_an_earlier_phase_longest_than_the_running_one() {
    let store = DocumentStore::default();
    let before = store.contention();
    let held = store.lock("quick").await;
    held.retag("slow-and-still-running");
    crate::rt::sleep(std::time::Duration::from_millis(120)).await;

    let line = describe_documents_contention(&before, &store.contention());
    assert!(line.contains("held by slow-and-still-running"), "{line}");
    assert!(
        !line.contains("longest phase"),
        "the running phase has outlasted every finished one, so `at this \
             point` is already the longest: {line}",
    );

    // Once it ends, the same phase becomes the mark like any other.
    held.retag("next");
    let line = describe_documents_contention(&before, &store.contention());
    assert!(
        line.contains("longest phase so far slow-and-still-running"),
        "{line}",
    );
}

/// A barrier that catches up *during* the sample window must not be
/// reported, and must not burn the rate limiter's slot.
///
/// The reporter sleeps for `DOCUMENTS_CONTENTION_SAMPLE_GAP` between its
/// two contention samples, which reopens the same race the entry check
/// closes: the stalled edit can land while it sleeps. Announcing a
/// permanent wedge that
/// is already over is bad on its own; recording that position in the rate
/// limiter is worse, because it would suppress the next *genuine* stall
/// there.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_barrier_that_catches_up_during_the_sample_window_is_not_reported() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///w/caught-up-midsample.tcl").unwrap();

    // A genuinely held turn, so the reporter gets past its entry check.
    let ticket = backend.edit_order.take_ticket("didOpen", &uri);
    let turn = backend.edit_order.wait_turn(ticket).await;
    let queued = backend.edit_order.take_ticket("didChange", &uri);
    let target = backend.edit_order.settle_target();

    let reporter = {
        let backend = Arc::clone(&backend);
        tokio::spawn(async move { backend.report_edit_barrier_stall(target).await })
    };

    // Let the reporter reach its sleep, then let the barrier catch up while
    // it is still in there.
    crate::rt::sleep(DOCUMENTS_CONTENTION_SAMPLE_GAP / 5).await;
    drop(turn);
    drop(queued);

    crate::rt::timeout(std::time::Duration::from_secs(5), reporter)
        .await
        .expect("the reporter must not hang")
        .expect("the reporter task must not panic");

    assert_eq!(
        backend
            .edit_barrier_stall_reported
            .load(std::sync::atomic::Ordering::Acquire),
        u64::MAX,
        "an edit that landed during the sample window must leave the rate \
             limiter untouched — recording it would suppress the next real stall \
             at this position (#1664's lesson, reopened by the sample sleep and \
             closed again here)",
    );
}

/// A stalled barrier report carries the index clause.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_stall_report_names_the_workspace_index_holder() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///w/index-holder.tcl").unwrap();
    let index = backend.workspace_index.write().await;
    index.retag("test: holding the index");

    let ticket = backend.edit_order.take_ticket("didClose", &uri);
    let _turn = backend.edit_order.wait_turn(ticket).await;
    let queued = backend.edit_order.take_ticket("didChange", &uri);
    let target = backend.edit_order.settle_target();
    crate::rt::timeout(
        std::time::Duration::from_secs(5),
        backend.report_edit_barrier_stall(target),
    )
    .await
    .expect("the reporter must return promptly rather than block");
    assert_ne!(
        backend
            .edit_barrier_stall_reported
            .load(std::sync::atomic::Ordering::Acquire),
        u64::MAX,
        "a genuinely held turn is reported",
    );
    let clause = backend.workspace_index.contention();
    assert!(
        clause.contains("held for write by") && clause.contains("`test: holding the index`"),
        "the clause the report embeds names the holder: {clause}",
    );
    drop(queued);
    drop(index);
}

/// The waiter clause must state the right verdict for each telemetry shape.
///
/// The whole point of the shim is that a human reads the verdict off the
/// stall line, so the mapping from counters to words is behaviour, not
/// cosmetics: woken-after-last-poll must read as a driving problem,
/// and never-woken as a lost wake, without guessing which scheduler layer
/// failed to drive it.
#[test]
fn the_waiter_clause_states_the_verdict() {
    let base = WaiterSnapshot {
        telemetry_id: 1,
        site: "did_open",
        parked_for: std::time::Duration::from_secs(18),
        polls: 1,
        since_last_poll: std::time::Duration::from_secs(18),
        wakes: 0,
        since_last_wake: None,
        #[cfg(not(target_family = "wasm"))]
        task_id: None,
    };

    let never_woken = describe_documents_waiters(&[base], false);
    assert!(
        never_woken.contains("lost or never sent"),
        "a never-woken waiter is the lost-wake verdict: {never_woken}",
    );

    let woken_not_polled = describe_documents_waiters(
        &[WaiterSnapshot {
            wakes: 1,
            since_last_wake: Some(std::time::Duration::from_secs(9)),
            ..base
        }],
        false,
    );
    assert!(
        woken_not_polled.contains("never polled again"),
        "a wake newer than the last poll is the not-being-driven verdict: \
             {woken_not_polled}",
    );

    let nobody = describe_documents_waiters(&[], false);
    assert!(
        nobody.contains("No task is in the map's contended-acquire path"),
        "an empty registry must say so rather than staying silent: {nobody}",
    );

    // Behind a HELD map, zero wakes is the expected state, not evidence:
    // the holder has not released, so no wake was ever due. The run-1
    // poll-shim capture printed the lost-wake wording for four such
    // victims, which invited suspicion of the mutex for waiters that were
    // merely queued behind a stuck holder.
    let held_victim = describe_documents_waiters(&[base], true);
    assert!(
        held_victim.contains("a victim of the holder above"),
        "a waiter behind a held map must be framed as a victim: {held_victim}",
    );
    assert!(
        !held_victim.contains("lost or never sent"),
        "a waiter behind a held map must not be given the lost-wake verdict: \
             {held_victim}",
    );
}
