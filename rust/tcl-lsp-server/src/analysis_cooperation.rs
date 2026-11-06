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

//! Transport progress scheduling; this never supplies source or execution facts.
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tcl_compiler::analysis_control::{AnalysisCheckpoint, AnalysisControl};

/// Only the transport advances routing progress or selects terminal state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Progress {
    generation: u64,
    stopped: bool,
    reason: TransportProgress,
}
/// Why a suspended analysis may leave its progress wait.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportProgress {
    /// The driver routed another input turn; a complete retry may proceed.
    Routed,
    /// The driver consumed readable bytes, leaving only an incomplete frame.
    DrainedIncomplete,
    /// The driver selected terminal completion; no analysis may restart.
    Stopped,
}
/// A single transport's scheduling owner; probes never consume input.
pub struct AnalysisCooperation {
    progress: tokio::sync::watch::Sender<Progress>,
    buffered_ready: AtomicBool,
    readiness: Box<dyn Fn() -> bool + Send + Sync>,
}
impl AnalysisCooperation {
    /// `readiness` is nonblocking and must not dispatch or consume input.
    pub fn new(readiness: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        Self {
            progress: tokio::sync::watch::channel(Progress {
                generation: 0,
                stopped: false,
                reason: TransportProgress::Routed,
            })
            .0,
            buffered_ready: AtomicBool::new(false),
            readiness: Box::new(readiness),
        }
    }
    /// Snapshot actual transport progress before starting a job.
    pub fn ticket(&self) -> u64 {
        self.progress.borrow().generation
    }
    /// Complete decoded frames count even when the raw fd has gone quiet.
    pub fn set_buffered_ready(&self, ready: bool) {
        self.buffered_ready.store(ready, Ordering::Release);
    }
    fn advance(&self, reason: TransportProgress) {
        self.progress.send_modify(|state| {
            if !state.stopped {
                state.generation = state.generation.saturating_add(1);
                state.reason = reason;
            }
        });
    }
    /// Called after complete frames have been routed and requests admitted.
    pub fn routed(&self) {
        self.advance(TransportProgress::Routed);
    }
    /// The sole reader consumed bytes, but needs more to complete a frame.
    /// This permits one quiet retry without calling a partial frame routed.
    pub fn drained_incomplete(&self) {
        self.advance(TransportProgress::DrainedIncomplete);
    }
    /// Stop is sticky and published before any deferred task is woken.
    pub fn stop(&self) {
        self.progress.send_modify(|state| state.stopped = true);
    }
    /// New jobs use the same coordinator; stopped jobs cannot be resurrected.
    pub fn control(self: &Arc<Self>) -> AnalysisControl {
        let owner = Arc::clone(self);
        let probes = AtomicUsize::new(0);
        AnalysisControl::new(move |at| {
            let stopped = owner.progress.borrow().stopped;
            let buffered = owner.buffered_ready.load(Ordering::Acquire);
            if stopped || buffered {
                return true;
            }
            let visit = probes.fetch_add(1, Ordering::Relaxed);
            // Phase boundaries are immediate; inner visits use a fixed work quota.
            (matches!(
                at,
                AnalysisCheckpoint::Start
                    | AnalysisCheckpoint::Realm
                    | AnalysisCheckpoint::Diagnostics
            ) || visit.is_multiple_of(64))
                && (owner.readiness)()
        })
    }
    /// Watch retains a version even when progress precedes this await.
    pub async fn wait_after(&self, ticket: u64) -> TransportProgress {
        let mut progress = self.progress.subscribe();
        loop {
            let observed = *progress.borrow_and_update();
            if observed.stopped {
                return TransportProgress::Stopped;
            }
            if observed.generation != ticket {
                return observed.reason;
            }
            if progress.changed().await.is_err() {
                return TransportProgress::Stopped;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::FutureExt as _;
    #[tokio::test]
    async fn progress_before_wait_is_retained_and_stop_dominates_later_wakes() {
        let owner = AnalysisCooperation::new(|| false);
        let ticket = owner.ticket();
        owner.routed();
        assert_eq!(owner.wait_after(ticket).await, TransportProgress::Routed);
        let current = owner.ticket();
        assert!(owner.wait_after(current).now_or_never().is_none());
        owner.stop();
        owner.routed();
        assert_eq!(owner.wait_after(current).await, TransportProgress::Stopped);
    }
    #[test]
    fn buffered_frames_interrupt_without_reading_a_quiet_fd() {
        let reads = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&reads);
        let owner = Arc::new(AnalysisCooperation::new(move || {
            count.fetch_add(1, Ordering::Relaxed);
            false
        }));
        owner.set_buffered_ready(true);
        let result = tcl_compiler::analyser::Analyser::new().try_analyse_owned(
            "set x 1",
            "tcl",
            None,
            owner.control(),
        );
        assert!(result.is_err());
        assert_eq!(reads.load(Ordering::Relaxed), 0);
    }
}
