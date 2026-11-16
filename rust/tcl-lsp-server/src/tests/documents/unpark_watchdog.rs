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

//! The watchdog's nudge trigger and step.

use super::*;

/// The nudge triggers only for a stale waiter on a free map.
///
/// The watchdog acts on production state, so a wrong predicate is either a
/// evidence probe that never fires (the wedge stays unobserved) or one that
/// fires on healthy states (pokes and log noise on every long but
/// legitimate hold). The trigger is pinned against its nearest legitimate
/// neighbour.
#[test]
fn the_nudge_fires_on_impossible_shapes_only() {
    let held = HeldFor {
        site: "deliver_if_current",
        held: std::time::Duration::from_secs(5),
        in_phase: std::time::Duration::from_secs(5),
        longest_phase: None,
    };
    let waiter = |parked_ms: u64| WaiterSnapshot {
        telemetry_id: 2,
        site: "did_open",
        parked_for: std::time::Duration::from_millis(parked_ms),
        polls: 1,
        since_last_poll: std::time::Duration::from_millis(parked_ms),
        wakes: 0,
        since_last_wake: None,
        #[cfg(not(target_family = "wasm"))]
        task_id: None,
    };
    let contention = |held_by: Option<HeldFor>| DocumentsContention {
        held_by,
        last: None,
        acquisitions: 1,
    };

    // A stale waiter on a FREE map. Its neighbour — the same
    // waiter behind a HELD map — is every long but legitimate hold, and
    // must not nudge.
    assert_eq!(
        nudge_reason(&contention(None), &[waiter(4100)]),
        Some(NudgeTarget {
            telemetry_id: 2,
            polls: 1,
        }),
        "a fair mutex hands over immediately; a stale waiter on a free map \
             was never resumed",
    );
    assert_eq!(
        nudge_reason(&contention(Some(held)), &[waiter(60_000)]),
        None,
        "a waiter behind a held map is a victim of the holder, however \
             stale — nudging would fire on every long legitimate hold (#1678)",
    );
    assert_eq!(
        nudge_reason(&contention(None), &[waiter(500)]),
        None,
        "a fresh waiter on a free map is an ordinary handoff in progress",
    );
    assert_eq!(
        nudge_reason(&contention(None), &[]),
        None,
        "an idle map never nudges",
    );
}

/// The watchdog step nudges a wedged-looking store and leaves a healthy
/// one alone — driven directly, as the thread loop drives it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_watchdog_step_nudges_only_a_wedged_store() {
    let store = Arc::new(DocumentStore::default());
    let handle = tokio::runtime::Handle::current();
    let mut log = Vec::new();

    // Healthy: idle store, no nudge, nothing logged.
    assert!(
        !unpark_watchdog_step(&store, &handle, &mut log),
        "an idle store must not be nudged",
    );
    assert!(log.is_empty(), "no action, no log line");

    // Wedged-looking: a waiter registration on a free map, backdated past
    // the evidence threshold. There is deliberately no acquire future to
    // resume, so the outcome must report that the external spawns did not
    // recover the exact target.
    let old = crate::rt::Instant::now() - NUDGE_WAITER_THRESHOLD * 2;
    let telemetry = Arc::new(std::sync::Mutex::new(WaiterTelemetry {
        site: "did_open",
        parked_since: old,
        polls: 1,
        last_poll: old,
        wakes: 0,
        last_wake: None,
        #[cfg(not(target_family = "wasm"))]
        task_id: None,
    }));
    store.tracking().waiters.push(telemetry);

    let nudges_before = UNPARK_NUDGES.load(std::sync::atomic::Ordering::Relaxed);
    // Run the step off the runtime, as the real watchdog thread does — it
    // sleeps 500ms inline, which must not block a worker in this test.
    let stepped = {
        let store = Arc::clone(&store);
        let handle = handle.clone();
        tokio::task::spawn_blocking(move || {
            let mut log = Vec::new();
            let stepped = unpark_watchdog_step(&store, &handle, &mut log);
            (stepped, log)
        })
        .await
        .expect("watchdog step must not panic")
    };
    assert!(stepped.0, "a stale waiter on a free map must be nudged");
    let logged = String::from_utf8(stepped.1).expect("log is utf8");
    assert!(
        logged.contains("[nudge] waiter parked on a free map"),
        "the nudge must log its reason and evidence: {logged}",
    );
    assert!(
        logged.contains("[nudge] outcome: STILL WEDGED"),
        "the nudge must log its outcome — the outcome line is the built-in \
             falsifier for the lost-unpark frame: {logged}",
    );
    assert_eq!(
        UNPARK_NUDGES.load(std::sync::atomic::Ordering::Relaxed),
        nudges_before + 1,
        "each nudge counts",
    );
}
