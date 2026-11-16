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

//! The open-document map's holder record, phases and waiter telemetry.

use super::*;

/// The stall line must name **who holds** the open-document map, not only
/// which lock the turn holder is waiting on.
///
/// This is the other half of the chain. The phase marker says a wedged
/// `did_change` sits at `did_change: documents.lock`; that names what it
/// wants, not who has it. Inferring the holder from an adjacent
/// `diagnostics.publish.enqueued` marker with no matching completion is an
/// inference, and a wedge need not leave so tidy a trail.
///
/// Constructed rather than raced: the test takes the map itself under a
/// known tag and holds it, which is exactly the state a real holder is in.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_open_document_map_names_its_holder() {
    let backend = test_backend();
    assert!(
        backend.documents.holder().is_none(),
        "a free map must report no holder, or a stall line would accuse a \
             call site that has already finished",
    );

    {
        let _held = backend.documents.lock("a-known-test-site").await;
        let held = backend.documents.holder().expect(
            "a held map must name its holder — without this the #1657 \
                         stall line can only say which lock is wanted, never who has it",
        );
        assert_eq!(
            held.site, "a-known-test-site",
            "the holder must be the site that actually took the lock",
        );
        // Sanity that the clock is the shim and is running, not a default.
        assert!(
            held.held < std::time::Duration::from_secs(60),
            "the hold age must be measured from the acquisition, not from an epoch",
        );
        // A hold still in its first phase reports the two ages as equal —
        // one clock reading serves both at acquisition.
        assert!(
            held.in_phase <= held.held,
            "the current phase cannot predate the hold that owns it",
        );
    }

    assert!(
        backend.documents.holder().is_none(),
        "dropping the guard must clear the holder; a tag that outlived its \
             guard would blame the previous holder for the next wedge",
    );

    // And the tag tracks a second, different holder rather than sticking.
    {
        let _held = backend.documents.lock("a-second-site").await;
        assert_eq!(
            backend.documents.holder().expect("held").site,
            "a-second-site",
            "each acquisition must overwrite the tag",
        );
    }
}

/// The map must distinguish "nobody wants it" from "everyone but the
/// waiter is getting it".
///
/// A wedge can leave the barrier holder parked on `documents.lock()`
/// while the map reads as *free*. One sample cannot tell those apart, and
/// they need different fixes: a lost wakeup is a scheduling fault, whereas a
/// map being taken repeatedly past a waiter means the waiter is not in the
/// queue at all. The acquisition counter is what separates them, so it has
/// to actually count.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_document_map_counts_acquisitions_and_remembers_its_last_holder() {
    let store = DocumentStore::default();

    let idle = store.contention();
    assert_eq!(idle.acquisitions, 0, "a fresh map has never been taken");
    assert!(idle.held_by.is_none(), "a fresh map is free");
    assert!(
        idle.last.is_none(),
        "a fresh map has no previous holder to blame",
    );

    {
        let _held = store.lock("first-site").await;
        let now = store.contention();
        assert_eq!(now.acquisitions, 1, "taking the map must count");
        assert_eq!(
            now.held_by.expect("held").site,
            "first-site",
            "a held map names its holder",
        );
    }

    // Released: free, but the previous holder is still nameable — the thing
    // the capture could not tell me.
    let after = store.contention();
    assert!(after.held_by.is_none(), "the guard released the map");
    assert_eq!(
        after.last.expect("a previous holder is remembered").0,
        "first-site",
        "a free map must still say who held it last, or a stall line that \
             finds it free has nothing to report at all",
    );

    // A second acquisition advances the counter: this is the reading that,
    // sampled twice during a stall, says the map is being taken *past* a
    // parked waiter rather than sitting idle.
    {
        let _held = store.lock("second-site").await;
    }
    let later = store.contention();
    assert_eq!(
        later.acquisitions, 2,
        "every acquisition must count, or a busy map is indistinguishable \
             from an untouched one and the lost-wakeup verdict is unfalsifiable",
    );
    assert_eq!(
        later.last.expect("previous holder").0,
        "second-site",
        "the remembered holder must be the most recent one",
    );
}

/// Retagging must restart the phase clock and leave the hold clock alone.
///
/// A line reading `the open-document map is held by cache_and_deliver:
/// publish send (70s)` cannot be acted on, because one age cannot say
/// whether the *publish send* is slow or whether a slow earlier phase had
/// already burned that time before the hold reached it. Retag overwrites
/// the site, so the earlier phase leaves no trace, and the two
/// possibilities want different fixes.
///
/// Both clocks together answer it: `held` is the whole hold, `in_phase` is
/// this step. `TurnHolder` carries the same pairing, so the invariant is
/// pinned here rather than left to be re-derived.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retagging_the_map_restarts_the_phase_clock_but_not_the_hold_clock() {
    let store = DocumentStore::default();
    let held = store.lock("first-phase").await;

    // A fresh hold is entirely within its first phase, so the two ages agree.
    let fresh = store.holder().expect("held");
    assert_eq!(fresh.site, "first-phase");
    assert!(
        fresh.held.saturating_sub(fresh.in_phase) < std::time::Duration::from_millis(50),
        "a hold still in its first phase must report the two ages as \
             effectively equal — one clock reading serves both at acquisition",
    );

    crate::rt::sleep(std::time::Duration::from_millis(120)).await;
    held.retag("second-phase");
    let after = store.holder().expect("still held");

    assert_eq!(after.site, "second-phase", "retag renames the holder");
    assert!(
        after.held >= std::time::Duration::from_millis(100),
        "the hold clock must survive a retag — it is the answer to 'how long \
             has the map been gone', which is what every waiter is blocked on",
    );
    assert!(
        after.in_phase < std::time::Duration::from_millis(100),
        "the phase clock must restart on a retag — without that, a long hold \
             reports only where it currently is, and a stall line cannot tell a \
             slow step from a step that was merely reached late (#1657)",
    );
    assert!(
        after.in_phase < after.held,
        "after a retag the current phase is strictly younger than the hold",
    );
}

/// The slowest phase of a hold must survive the retag that relabels it
/// away.
///
/// A line blaming `cache_and_deliver: publish send` for far longer than a
/// send's own budget allows means the time went to a phase that had
/// already been renamed. `site` and `phase_since` answer
/// only "is *this* step slow"; without a high-water mark nothing names an
/// earlier one.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_longest_phase_of_a_hold_survives_the_retag_that_renames_it() {
    let store = DocumentStore::default();
    let held = store.lock("slow-phase").await;

    // Nothing has finished yet, so there is no high-water mark to report
    // and the clause is suppressed rather than repeating the current phase.
    assert!(store.holder().expect("held").longest_phase.is_none());

    crate::rt::sleep(std::time::Duration::from_millis(120)).await;
    held.retag("fast-phase");

    let after = store.holder().expect("still held");
    let (site, elapsed) = after.longest_phase.expect("the first phase closed");
    assert_eq!(
        site, "slow-phase",
        "the high-water mark names the phase that burned the time, not the \
             one running now",
    );
    assert!(
        elapsed >= std::time::Duration::from_millis(100),
        "and carries its duration: {elapsed:?}",
    );
    assert!(
        after.in_phase < std::time::Duration::from_millis(100),
        "the current phase is young — which is exactly the reading that was \
             ambiguous before",
    );

    // A later, shorter phase does not displace it.
    held.retag("third-phase");
    assert_eq!(
        store
            .holder()
            .expect("still held")
            .longest_phase
            .map(|(s, _)| s),
        Some("slow-phase"),
    );
}

/// The phase running at release is a finished phase like any other.
///
/// A hold whose *last* step is the slow one never retags again, so without
/// closing the phase on drop the shape most worth catching would be the one
/// that leaves no trace.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_phase_running_at_release_is_recorded_too() {
    let store = DocumentStore::default();
    let held = store.lock("quick-phase").await;
    held.retag("slow-final-phase");
    crate::rt::sleep(std::time::Duration::from_millis(120)).await;
    drop(held);

    let contention = store.contention();
    assert!(contention.held_by.is_none(), "released");
    let (site, _, longest) = contention.last.expect("a departed holder");
    assert_eq!(site, "slow-final-phase");
    let (longest_site, elapsed) = longest.expect("the final phase closed on release");
    assert_eq!(longest_site, "slow-final-phase");
    assert!(
        elapsed >= std::time::Duration::from_millis(100),
        "{elapsed:?}",
    );
}

/// A contention snapshot must describe one state, not several.
///
/// Behind two mutexes and an atomic, the three readings could interleave:
/// read the holder, watch
/// that guard drop and another task acquire, then read the last-holder and
/// the count from *after* the change. The line then pairs one task's
/// identity with another's metadata — the exact confusion the discriminator
/// exists to remove. Dating every age from one clock reading does not fix
/// it; that fixes *when* the numbers were taken, not *which state* they
/// describe. Only a shared lock does, which is why all three now live
/// under one.
///
/// Atomicity itself is guaranteed by construction and is verified
/// structurally: reverting to split fields fails this test's concurrent
/// half. What is pinned here is the record's semantics, which a future
/// edit could break without touching the lock.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_contention_snapshot_never_mixes_two_states() {
    let store = Arc::new(DocumentStore::default());

    // The deterministic half: the transitions the stall line reads.
    let empty = store.contention();
    assert_eq!(empty.acquisitions, 0);
    assert!(empty.held_by.is_none() && empty.last.is_none());

    {
        let _a = store.lock("site-a").await;
        let snap = store.contention();
        assert_eq!(snap.held_by.expect("held").site, "site-a");
        assert_eq!(snap.acquisitions, 1, "the holder's own acquisition counts");
        assert!(
            snap.last.is_none(),
            "nothing has been released yet, so there is no previous holder",
        );
    }
    let snap = store.contention();
    assert!(snap.held_by.is_none(), "released");
    assert_eq!(snap.last.expect("remembered").0, "site-a");
    assert_eq!(snap.acquisitions, 1, "releasing is not an acquisition");

    {
        let _b = store.lock("site-b").await;
        let snap = store.contention();
        assert_eq!(snap.held_by.expect("held").site, "site-b");
        assert_eq!(
            snap.last.expect("remembered").0,
            "site-a",
            "the previous holder is the one before the current, not the current",
        );
        assert_eq!(snap.acquisitions, 2);
    }
}

/// The concurrent half of the snapshot-coherence pin: genuinely impossible
/// pairings, hammered.
///
/// Split from [`a_contention_snapshot_never_mixes_two_states`]'s
/// deterministic transition checks purely for the line-count lint; the two
/// are one argument, and the doc comment there carries it.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_contention_snapshot_survives_concurrent_hammering() {
    let store = Arc::new(DocumentStore::default());
    //
    // Every acquisition uses a site name used exactly once, ever. A site can
    // then never be both the current holder and the most recent previous
    // holder in any real state — that pairing requires reading `holder`
    // before a release and `last` after it, which is precisely the tear.
    //
    // The obvious version of this test (a fixed set of tags) does not work,
    // and quietly passes against a deliberately-torn `contention`: a task
    // that releases and immediately re-acquires is *legitimately* both the
    // holder and the previous holder, so the pairing proves nothing. The
    // uniqueness is what makes it proof.
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let next_tag = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let hammers: Vec<_> = (0..3)
        .map(|_| {
            let store = Arc::clone(&store);
            let stop = Arc::clone(&stop);
            let next_tag = Arc::clone(&next_tag);
            tokio::spawn(async move {
                while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                    let n = next_tag.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    // `&'static str` per acquisition. Leaked deliberately and
                    // bounded by the loop below; a few thousand short strings.
                    let site: &'static str = Box::leak(format!("hammer-{n}").into_boxed_str());
                    let held = store.lock(site).await;
                    for _ in 0..32 {
                        crate::rt::yield_now().await;
                    }
                    drop(held);
                    crate::rt::yield_now().await;
                }
            })
        })
        .collect();

    // Do not start the synchronous sampling loop until the real hammer
    // workers are demonstrably running. They keep acquiring and releasing
    // on the other runtime threads while sampling, preserving the
    // concurrent-transition proof without depending on initial scheduling.
    while store.contention().acquisitions < 100 {
        crate::rt::yield_now().await;
    }
    let mut previous = store.contention().acquisitions;
    let mut seen_both = 0_u32;
    for _ in 0..100_000 {
        let snap = store.contention();
        assert!(
            snap.acquisitions >= previous,
            "the acquisition count went backwards",
        );
        previous = snap.acquisitions;
        if let (Some(held), Some((last_site, _, _))) = (snap.held_by, snap.last) {
            seen_both += 1;
            assert_ne!(
                held.site, last_site,
                "this snapshot names a site as both the current holder and \
                     the previous one. Every acquisition uses a unique tag, so no \
                     real state can look like this — the fields were read at \
                     different moments, and a stall line built from them would \
                     name one task while describing another",
            );
            assert!(
                snap.acquisitions > 0,
                "a snapshot showing a holder must show its acquisition too",
            );
        }
    }

    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    for h in hammers {
        let _ = h.await;
    }
    assert!(
        seen_both > 0,
        "the hammer never caught the map held with a previous holder \
             recorded, so the concurrent half proved nothing",
    );
}

/// A contended acquire must be visible, counted, woken-counted, and gone
/// when it is over.
///
/// This is the poll-discriminator's ground truth. A wedge shows a waiter
/// parked on a free map; the verdict the stall line
/// draws — wake lost, versus woken-but-never-polled — is only as good as
/// these counters. A waiter that did not register would be invisible; polls
/// that did not count would read as "never polled" (the parent-starvation
/// verdict) for a future being polled constantly; wakes that did not count
/// would read as "wake lost" (the extraordinary claim) for a wake that
/// arrived.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_contended_map_acquire_reports_its_polls_and_wakes() {
    let store = Arc::new(DocumentStore::default());
    assert!(
        store.waiters().is_empty(),
        "no waiter exists before anything contends",
    );

    let held = store.lock("holder").await;
    assert!(
        store.waiters().is_empty(),
        "the fast path must not register a waiter — there is nothing \
             waiting, and a ghost entry would be accused by the next stall line",
    );

    let contender = {
        let store = Arc::clone(&store);
        tokio::spawn(async move {
            let _guard = store.lock("contender").await;
        })
    };

    // The contender parks. Its registration and at least one poll must be
    // visible while it waits.
    let parked = crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let waiters = store.waiters();
            if let [w] = waiters.as_slice()
                && w.site == "contender"
                && w.polls >= 1
            {
                return *w;
            }
            crate::rt::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("the parked contender must appear in the waiter registry");
    assert_eq!(
        parked.wakes, 0,
        "nothing has released the map, so no wake can have been recorded — \
             a phantom wake here would later disprove a real lost-wake verdict",
    );

    // Release. The fair mutex hands the lock to the queued contender, which
    // requires waking it — that wake must be counted before the waiter
    // finishes and deregisters. We can't sample the registry after the
    // handoff (the entry is gone, correctly), so sample the Arc the
    // registry shared: grab it while the waiter is still parked.
    let telemetry = {
        let tracking = store.tracking();
        Arc::clone(&tracking.waiters[0])
    };
    drop(held);
    crate::rt::timeout(std::time::Duration::from_secs(5), contender)
        .await
        .expect("the contender must acquire once the map is released")
        .expect("the contender task must not panic");

    assert!(
        store.waiters().is_empty(),
        "a completed acquire must deregister — a stale entry would be \
             accused by every later stall line",
    );
    let t = telemetry
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(
        t.wakes >= 1,
        "releasing the map woke the queued waiter, and that wake must be \
             counted: with it missed, a stall line would report 'never woken' — \
             the lost-wake verdict — about a wake that demonstrably arrived",
    );
    assert!(
        t.polls >= 2,
        "the wake leads to a final successful poll, which must be counted \
             like the first",
    );
}

/// A waiter cancelled mid-wait must deregister.
///
/// Handler futures are dropped wholesale (a closed connection, an aborted
/// request), taking any parked acquire with them. The registration is
/// removed by a drop guard precisely so that this case cannot leave a ghost
/// waiter — an entry that would sit in the registry forever and be accused
/// by every subsequent stall line as a decades-old parked task.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cancelled_map_acquire_leaves_no_ghost_waiter() {
    let store = Arc::new(DocumentStore::default());
    let held = store.lock("holder").await;

    {
        let mut acquire = Box::pin(store.lock("doomed"));
        assert!(
            crate::rt::timeout(std::time::Duration::from_millis(50), &mut acquire)
                .await
                .is_err(),
            "the acquire must park behind the held map, or dropping it \
                 proves nothing",
        );
        assert_eq!(
            store.waiters().len(),
            1,
            "the parked acquire must be registered",
        );
        // `acquire` drops here — cancellation.
    }

    assert!(
        store.waiters().is_empty(),
        "a cancelled acquire must deregister its waiter — the drop guard is \
             the only thing between this and a permanent ghost entry (#1657)",
    );
    drop(held);
}
