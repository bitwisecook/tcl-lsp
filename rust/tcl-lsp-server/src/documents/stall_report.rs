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

//! The stall line: who holds the document map, who waits and where the edit barrier stopped.

use super::edit_order::{EDIT_BARRIER_STALL_LOG, EDIT_BARRIER_STALL_WARN};
use super::{DocumentsContention, PhaseSpan, WaiterSnapshot};
use crate::{Backend, MessageType};

/// Render the map's waiter telemetry into the stall line's
/// poll-discrimination clause.
///
/// This is the reading that separates the three remaining stories for a waiter
/// parked on a free map:
///
/// * **woken after its last poll** — the wake arrived and nothing polled the
///   future again. This proves the exact waiter is not being driven; it does not
///   guess where between wake delivery and polling it was lost.
/// * **never woken since parking** — the wake was genuinely lost (or never
///   sent).
///
/// Pure formatting over snapshots, split out for the same reason as
/// [`describe_documents_contention`].
pub(in super::super) fn describe_documents_waiters(
    waiters: &[WaiterSnapshot],
    map_held: bool,
) -> String {
    if waiters.is_empty() {
        return "No task is in the map's contended-acquire path (its instrumented slow path)."
            .to_owned();
    }
    let mut out = String::new();
    for w in waiters {
        use std::fmt::Write as _;
        // A waiter behind a HELD map has not been woken because no wake was
        // ever due — the holder has not released. Framing that as a lost wake
        // invites the reader to suspect the mutex when the waiters are merely
        // victims of the holder.
        // The wake analysis below is only evidence when the map is free.
        if map_held {
            let _ = write!(
                out,
                "The map's acquire queue holds {} (parked {:.1}s, polled {} time(s), \
                 woken {} time(s) — a victim of the holder above, no wake due until it \
                 releases). ",
                w.site,
                w.parked_for.as_secs_f64(),
                w.polls,
                w.wakes,
            );
            continue;
        }
        let wake = match w.since_last_wake {
            Some(age) if age < w.since_last_poll => format!(
                "woken {:.1}s ago, AFTER its last poll — the wake arrived and the future was \
                 never polled again, so its task is not being driven",
                age.as_secs_f64(),
            ),
            Some(age) => format!(
                "last woken {:.1}s ago, before its last poll — no wake has arrived since it \
                 parked, so the wake it is waiting for was lost or never sent",
                age.as_secs_f64(),
            ),
            None => "never woken at all — the wake it is waiting for was lost or never \
                     sent"
                .to_owned(),
        };
        let _ = write!(
            out,
            "The map's acquire queue holds {}: parked {:.1}s, polled {} time(s) (last \
             {:.1}s ago), woken {} time(s) ({wake}). ",
            w.site,
            w.parked_for.as_secs_f64(),
            w.polls,
            w.since_last_poll.as_secs_f64(),
            w.wakes,
        );
    }
    out
}

/// Render the two contention samples a stall report takes into one clause.
///
/// Split out of `Backend::report_edit_barrier_stall` to keep that function under
/// the line limit. It is pure formatting over two snapshots, with no bearing on
/// the lock discipline the reporter is describing, so unlike
/// `publish_diagnostics_result` — where splitting would put a critical
/// section's acquisition and release in different functions — there is nothing
/// here that reads better kept together.
pub(in super::super) fn describe_documents_contention(
    before: &DocumentsContention,
    after: &DocumentsContention,
) -> String {
    let taken = after.acquisitions.saturating_sub(before.acquisitions);
    if let Some(h) = after.held_by {
        // Three readings, never fewer: `held` says the hold is long,
        // `in_phase` says whether *this* step is what is long, and the
        // high-water mark names the slowest step already behind it — which a
        // retag would otherwise have relabelled away. The acquisition counter
        // is deliberately not printed here — nobody else can acquire a held
        // map, so it is trivially zero.
        return format!(
            "the open-document map is held by {} — {:.1}s in total, {:.1}s at this point{}",
            h.site,
            h.held.as_secs_f64(),
            h.in_phase.as_secs_f64(),
            describe_longest_phase(h.longest_phase, Some(h.in_phase)),
        );
    }
    let last = after.last.map_or_else(
        || "never held".to_owned(),
        |(site, age, longest)| {
            format!(
                "last held by {site}, whose hold began {:.1}s ago{}",
                age.as_secs_f64(),
                describe_longest_phase(longest, None),
            )
        },
    );
    if taken == 0 {
        format!(
            "the open-document map is FREE and untouched across the sample window \
             ({last}), so a waiter parked on it was never woken — a lost wakeup, not \
             contention"
        )
    } else {
        format!(
            "the open-document map is free right now but was taken {taken} time(s) \
             during the sample window ({last}), so it is being acquired past a waiter \
             that should be ahead of it — the waiter is not in the queue rather than \
             the lock being stuck"
        )
    }
}

/// Render a hold's high-water phase as a trailing clause, or nothing when there
/// is no earlier phase worth naming.
///
/// `live` is the phase running now, where there is one. The mark deliberately
/// covers only *finished* phases, so a hold that is 70s into `publish send`
/// after a 1s earlier phase would otherwise read `70.0s at this point;
/// longest phase so far <earlier> (1.0s)` — false, and pointing an
/// investigation at the wrong step. The clause is therefore
/// suppressed unless the mark actually outlasts the live phase; when it does
/// not, `in_phase` has already named the longest phase and repeating it adds a
/// number to squint at rather than a fact. A hold still in its first phase has
/// no mark at all, and a released one has no live phase — its final phase was
/// folded in on drop, so the mark is the true longest.
fn describe_longest_phase(longest: Option<PhaseSpan>, live: Option<std::time::Duration>) -> String {
    let Some((site, elapsed)) = longest else {
        return String::new();
    };
    if live.is_some_and(|live| live >= elapsed) {
        return String::new();
    }
    format!(
        "; longest phase so far {site} ({:.1}s)",
        elapsed.as_secs_f64()
    )
}

/// How long [`Backend::report_edit_barrier_stall`] waits between its two
/// [`DocumentStore::contention`] samples.
///
/// Long enough that a map under real traffic is provably taken at least once,
/// short enough to be lost in the noise of a stall that has already lasted
/// [`EDIT_BARRIER_STALL_WARN`]. Paid only on the reporting path, which runs once
/// per stuck barrier position and never on a healthy server.
pub(in super::super) const DOCUMENTS_CONTENTION_SAMPLE_GAP: std::time::Duration =
    std::time::Duration::from_millis(250);

impl Backend {
    /// Announce that the document-sync barrier has stopped advancing.
    ///
    /// Rate-limited to one line per stuck position: in a real wedge *every*
    /// request handler arrives here, and a log storm would push the very lines
    /// a reader needs out of the captured tail. Reporting again once the
    /// sequence has moved on is intentional — a barrier that stalls repeatedly
    /// at successive positions is a different story from one that stopped dead.
    ///
    /// # The barrier is re-read before anything is claimed
    ///
    /// The caller's timeout can win its race by a hair: the wait expires, and
    /// the edit it was waiting for lands before this runs. Reporting on the
    /// strength of "the timeout fired" would then announce a permanent wedge
    /// that is already over — with no holder and zero notifications queued,
    /// which is the *ticket-loss* shape and so actively misleading.
    ///
    /// Worse than the noise is what it would do to the rate limiter. Recording
    /// that position would make a later, genuine stall at the same position
    /// suppress itself, so the false report could hide the real one. Both
    /// failures land in the one diagnosis stream a wedge leaves behind, so the
    /// re-read comes first and the suppression slot is only ever written for a
    /// stall that is real at the moment it is claimed.
    pub(in super::super) async fn report_edit_barrier_stall(&self, target: u64) {
        if self.edit_order.served() >= target {
            return;
        }
        let holder = self
            .edit_order
            .holder
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        // Naming the holder is the difference between "the barrier stopped" and
        // a diagnosis. No holder at all is itself informative: nobody is in the
        // turn, so the sequence is stuck on a ticket whose handler never
        // reached its grant.
        let culprit = holder.map_or_else(
            || {
                "held by nobody — the sequence is stuck on a ticket whose handler never \
                 reached its grant"
                    .to_owned()
            },
            |h| {
                format!(
                    "held by {} for {:.1}s on {} (ticket {}), suspended at `{}` for {:.1}s",
                    h.what,
                    h.since.elapsed().as_secs_f64(),
                    h.uri,
                    h.ticket,
                    h.phase,
                    h.phase_since.elapsed().as_secs_f64(),
                )
            },
        );
        // A second, independent reading of the same moment. `did_open` /
        // `did_change` both reach `db_set_source` inside the turn, and salsa's
        // `cancel_others` blocks there until every other database snapshot has
        // been dropped, so an outstanding snapshot older than the stall is the
        // one thing that could hold that call.
        //
        // Stated as evidence, not as a verdict. The census counts live database
        // clones; it cannot see whether the holder is in `set_text` at all — the
        // phase marker above is what answers that, and the two are printed
        // together precisely so a reader can check them against each other. A
        // stale-looking snapshot next to a phase that never reached
        // `db_set_source` means the snapshot is a bystander.
        let census = self.db.snapshot_report();
        let waiting_on = match census.oldest {
            Some((site, age)) => format!(
                "{} salsa snapshot(s) are outstanding; the oldest was taken by {site} {:.1}s ago",
                census.outstanding,
                age.as_secs_f64(),
            ),
            None => "no salsa snapshot is outstanding, so nothing can be parked in \
                     `cancel_others`"
                .to_owned(),
        };
        // The third reading, and the one that closes the chain. The phase marker
        // says which lock the turn holder is *waiting* on; this says who is
        // *holding* it. Inferring the holder from adjacent log markers is
        // guesswork that depends on the wedge leaving a tidy trail.
        //
        // Two samples a moment apart, because a free map is ambiguous on its
        // own: "nobody wants it" and "everybody is taking turns while the
        // barrier holder is skipped" look identical in one reading. The gap is
        // short enough not to extend a stall anyone is waiting on and long
        // enough for a busy map to move.
        let before = self.documents.contention();
        crate::rt::sleep(DOCUMENTS_CONTENTION_SAMPLE_GAP).await;
        let after = self.documents.contention();
        // Re-read the barrier after the sample window, for the same reason it
        // is re-read after `edits_settled`'s timeout: the edit this report is
        // about may have landed while we slept. Announcing a
        // permanent wedge that is already over is bad on its own, and burning
        // the rate limiter's slot on that stale report is worse — it would
        // suppress the next *genuine* stall at this position.
        //
        // So the slot is claimed only here, after the barrier has been shown to
        // still be stuck. Claiming it earlier would deduplicate concurrent
        // reporters but pay for that with exactly the suppression above;
        // claiming it here deduplicates just as well, since the swap is still
        // the gate — the other reporters simply sleep first and then find the
        // slot taken.
        let serving = self.edit_order.served();
        if serving >= target {
            return;
        }
        if self
            .edit_barrier_stall_reported
            .swap(serving, std::sync::atomic::Ordering::AcqRel)
            == serving
        {
            return;
        }
        let documents_holder = describe_documents_contention(&before, &after);
        let waiters =
            describe_documents_waiters(&self.documents.waiters(), after.held_by.is_some());
        // The fourth reading: a map holder parked on the workspace index is
        // only explained by who holds the index against it.
        let index = self.workspace_index.contention();
        let db = self.db.contention();
        let db_files = self.db_files.contention();
        self.client
            .log_message(
                MessageType::WARNING,
                format!(
                    "{EDIT_BARRIER_STALL_LOG} for {}s: now_serving={serving}, waiting for \
                     {target}, so {} document-sync notification(s) are queued behind it. \
                     The turn is {culprit}. {documents_holder}. {waiters} {waiting_on}. \
                     Workspace index: [{index}]. \
                     Salsa store contention: [{db}] [{db_files}]. \
                     Every request handler is blocked on this barrier and the server will \
                     answer nothing until it moves (issue #1657).",
                    EDIT_BARRIER_STALL_WARN.as_secs(),
                    target.saturating_sub(serving),
                ),
            )
            .await;
    }
}
