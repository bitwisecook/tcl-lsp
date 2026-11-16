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

//! The opt-in watchdog that nudges a waiter parked on a free document map.

use super::store::DocumentStore;
use super::{DocumentsContention, WaiterSnapshot};
use crate::{Arc, Backend};

/// How long a waiter may sit parked on a FREE map before the watchdog nudges.
/// A held map never triggers this: waiters behind a holder are victims, and a
/// long but legitimate hold must not cause false nudges.
#[cfg(not(target_family = "wasm"))]
pub(in super::super) const NUDGE_WAITER_THRESHOLD: std::time::Duration =
    std::time::Duration::from_secs(4);

/// Nudges fired since startup, for tests and post-mortem correlation.
#[cfg(not(target_family = "wasm"))]
pub static UNPARK_NUDGES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The exact telemetry registration a watchdog experiment acts on.
#[cfg(not(target_family = "wasm"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct NudgeTarget {
    pub(in super::super) telemetry_id: usize,
    pub(in super::super) polls: u64,
}

#[cfg(not(target_family = "wasm"))]
impl NudgeTarget {
    fn progressed(self, waiters: &[WaiterSnapshot]) -> bool {
        waiters
            .iter()
            .find(|waiter| waiter.telemetry_id == self.telemetry_id)
            .is_none_or(|waiter| waiter.polls > self.polls)
    }
}

/// Why the watchdog would nudge right now, if it would.
///
/// Pure over the sampled telemetry, so the trigger is unit-tested exactly as
/// the stall-line verdict is. A waiter parked on a **free** map past
/// [`NUDGE_WAITER_THRESHOLD`] is anomalous: a fair mutex with a queued waiter
/// hands over immediately. The external no-op spawn remains an opt-in evidence
/// experiment, not a recovery guarantee.
#[cfg(not(target_family = "wasm"))]
pub(in super::super) fn nudge_reason(
    contention: &DocumentsContention,
    waiters: &[WaiterSnapshot],
) -> Option<NudgeTarget> {
    if contention.held_by.is_none()
        && let Some(waiter) = waiters
            .iter()
            .find(|w| w.parked_for > NUDGE_WAITER_THRESHOLD)
    {
        return Some(NudgeTarget {
            telemetry_id: waiter.telemetry_id,
            polls: waiter.polls,
        });
    }
    None
}

/// One watchdog step: sample, decide, nudge, verify, report. Returns whether
/// it nudged. Factored out of the thread loop so tests drive it directly.
#[cfg(not(target_family = "wasm"))]
pub(in super::super) fn unpark_watchdog_step(
    store: &DocumentStore,
    handle: &tokio::runtime::Handle,
    log: &mut dyn std::io::Write,
) -> bool {
    let (contention, waiters) = store.watchdog_snapshot();
    let Some(target) = nudge_reason(&contention, &waiters) else {
        return false;
    };
    UNPARK_NUDGES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    // Plain facts, not the stall line's two-sample clauses — a nudge takes one
    // sample, and borrowing wording that claims a sample window would be the
    // instrument overclaiming.
    let holder = contention.held_by.map_or_else(
        || "map free".to_owned(),
        |h| format!("map held by {} ({:.1}s)", h.site, h.held.as_secs_f64()),
    );
    let target_task = waiters
        .iter()
        .find(|waiter| waiter.telemetry_id == target.telemetry_id)
        .and_then(|waiter| waiter.task_id)
        .map_or_else(|| "unknown".to_owned(), |id| id.to_string());
    let _ = writeln!(
        log,
        "[nudge] waiter parked on a free map: target task {target_task}; {holder}; {} waiter(s) parked. Spawning no-op tasks \
         from outside the runtime.",
        waiters.len(),
    );
    // The poke: an external spawn takes the remote/inject path, which unparks
    // a worker. Four, in case the first is consumed by an unrelated queue.
    for _ in 0..4 {
        handle.spawn(async {});
    }
    std::thread::sleep(std::time::Duration::from_millis(500));
    let (_, after_w) = store.watchdog_snapshot();
    if target.progressed(&after_w) {
        let _ = writeln!(
            log,
            "[nudge] outcome: target progressed — the exact telemetry registration polled \
             or deregistered after the external spawns",
        );
    } else {
        let _ = writeln!(
            log,
            "[nudge] outcome: STILL WEDGED — the exact telemetry registration did not poll \
             500ms after external spawns; the experiment did not recover it",
        );
        // Evidence builds correlate the exact waiter task ID with every dump
        // entry. A missing entry is reported as absence, not interpreted as a
        // scheduler state bit: Tokio may skip an already-notified task. Requires
        // RUSTFLAGS="--cfg tokio_unstable --cfg tokio_taskdump" (local
        // Linux evidence builds only; inert otherwise). Once per process.
        #[cfg(all(target_os = "linux", tokio_unstable, tokio_taskdump))]
        {
            let target_task_id = after_w
                .iter()
                .find(|waiter| waiter.telemetry_id == target.telemetry_id)
                .and_then(|waiter| waiter.task_id);
            try_taskdump(handle, log, target_task_id);
        }
    }
    true
}

/// Dump the tasks Tokio can trace and correlate them with the exact target ID.
///
/// Linux evidence builds only:
/// `RUSTFLAGS="--cfg tokio_unstable --cfg tokio_taskdump" cargo build ...`.
/// The custom cfg also enables Tokio's required `taskdump` Cargo feature via
/// the target dependency in `Cargo.toml`. Never enabled in CI. Bounded: once
/// per process, five-second cap.
///
/// Absence is deliberately not called a state verdict. Tokio's dump walk skips
/// an owned task when `notify_for_tracing` finds it already notified and it was
/// not drained from a scheduler queue; an empty trace likewise exposes no state
/// bits. The task ID is the only sound correlation this API supplies.
#[cfg(all(target_os = "linux", tokio_unstable, tokio_taskdump))]
fn try_taskdump(
    handle: &tokio::runtime::Handle,
    log: &mut dyn std::io::Write,
    target_task_id: Option<tokio::task::Id>,
) {
    static ONCE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if ONCE.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    let outcome = handle.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(5), handle.dump()).await
    });
    match outcome {
        Ok(dump) => {
            let mut target_seen = false;
            for (i, task) in dump.tasks().iter().enumerate() {
                let is_target = target_task_id == Some(task.id());
                target_seen |= is_target;
                let _ = writeln!(
                    log,
                    "[taskdump] task index={i} id={} target={is_target}:\n{}",
                    task.id(),
                    task.trace(),
                );
            }
            if let Some(target_task_id) = target_task_id
                && !target_seen
            {
                let _ = writeln!(
                    log,
                    "[taskdump] target task id={target_task_id} ABSENT from dump; Tokio may \
                     skip an already-notified task, so absence is evidence but not a state read",
                );
            }
        }
        Err(_) => {
            let _ = writeln!(
                log,
                "[taskdump] timed out after 5s — the runtime would not cooperate, \
                 which is itself evidence",
            );
        }
    }
}

/// Start the opt-in wedge-evidence watchdog on a plain std thread outside the
/// runtime it observes.
///
/// Samples the open-document map's telemetry every 250 ms; on a trigger shape
/// (see [`nudge_reason`]) it spawns no-op tasks into the runtime from outside
/// and logs the evidence and the outcome — to stderr always, and to the file
/// named by `TCL_LSP_NUDGE_LOG` when set, because the ext-host harness does
/// not reliably capture server stderr and the acceptance loop greps the file.
///
/// This is not a mitigation: an external unpark does not recover a wedged
/// waiter. Normal server startup therefore does not call this function; `main`
/// requires the explicit `TCL_LSP_WEDGE_EVIDENCE` opt-in. The poke is a
/// repeatable discriminator, and every action logs whether the exact target
/// progressed.
#[cfg(not(target_family = "wasm"))]
pub fn spawn_unpark_watchdog(backend: &Backend, handle: tokio::runtime::Handle) {
    let store = Arc::clone(&backend.documents);
    std::thread::Builder::new()
        .name("tcl-lsp-unpark-watchdog".into())
        .spawn(move || {
            let mut file = std::env::var("TCL_LSP_NUDGE_LOG").ok().and_then(|p| {
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(p)
                    .ok()
            });
            let mut last_nudge: Option<std::time::Instant> = None;
            loop {
                std::thread::sleep(std::time::Duration::from_millis(250));
                // Rate limit: at most one nudge burst per second.
                if last_nudge.is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(1)) {
                    continue;
                }
                let mut sink = NudgeLog {
                    file: file.as_mut(),
                };
                if unpark_watchdog_step(&store, &handle, &mut sink) {
                    last_nudge = Some(std::time::Instant::now());
                }
            }
        })
        .expect("spawning the unpark watchdog thread");
}

/// Tee for nudge lines: stderr always, plus the acceptance-loop file when set.
#[cfg(not(target_family = "wasm"))]
struct NudgeLog<'a> {
    pub(in super::super) file: Option<&'a mut std::fs::File>,
}

#[cfg(not(target_family = "wasm"))]
impl std::io::Write for NudgeLog<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let _ = std::io::stderr().write_all(buf);
        if let Some(f) = self.file.as_mut() {
            let _ = f.write_all(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let _ = std::io::stderr().flush();
        if let Some(f) = self.file.as_mut() {
            let _ = f.flush();
        }
        Ok(())
    }
}
