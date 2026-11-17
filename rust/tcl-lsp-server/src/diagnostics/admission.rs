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

//! How many documents' diagnostics analyses run at once.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Admits a bounded number of documents to analysis at once.
///
/// A pack change, a configuration reload or a workspace rescan reschedules
/// every open document. Each document's worker used to start its analysis
/// immediately, and every analysis runs on the blocking pool, so a reload over
/// a few hundred open documents ran a few hundred analyses side by side on a
/// handful of cores. The pool reached its thread cap, and stdin, which Tokio
/// reads on the same pool, stopped being read (#1657). A worker now waits here
/// for a permit after its debounce. It then analyses its document's latest
/// state, so a document that waited is never dropped, and an edit made while
/// it waited replaces the version it would have analysed.
pub(crate) struct AnalysisAdmission {
    permits: Arc<Semaphore>,
    in_flight: AtomicUsize,
    peak: AtomicUsize,
    admitted: AtomicUsize,
    #[cfg(test)]
    hold: Option<Arc<Semaphore>>,
}

impl AnalysisAdmission {
    /// Admit at most `bound` documents at once (at least one).
    pub(crate) fn with_bound(bound: usize) -> Self {
        Self {
            permits: Arc::new(Semaphore::new(bound.max(1))),
            in_flight: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
            admitted: AtomicUsize::new(0),
            #[cfg(test)]
            hold: None,
        }
    }

    /// The process's admission, one document per available core.
    pub(crate) fn shared() -> Arc<Self> {
        static SHARED: LazyLock<Arc<AnalysisAdmission>> = LazyLock::new(|| {
            let cores = std::thread::available_parallelism().map_or(2, std::num::NonZero::get);
            Arc::new(AnalysisAdmission::with_bound(cores))
        });
        Arc::clone(&SHARED)
    }

    /// Wait for a permit; the analysis is admitted while the guard lives.
    pub(crate) async fn admit(self: &Arc<Self>) -> Admitted {
        let permit = Arc::clone(&self.permits)
            .acquire_owned()
            .await
            .expect("the admission semaphore is never closed");
        let now = self.in_flight.fetch_add(1, Ordering::AcqRel) + 1;
        self.peak.fetch_max(now, Ordering::AcqRel);
        self.admitted.fetch_add(1, Ordering::AcqRel);
        #[cfg(test)]
        if let Some(hold) = &self.hold
            && let Ok(permit) = hold.acquire().await
        {
            permit.forget();
        }
        Admitted {
            _permit: permit,
            admission: Arc::clone(self),
        }
    }

    /// Documents admitted and not yet released.
    #[cfg(test)]
    pub(crate) fn in_flight(&self) -> usize {
        self.in_flight.load(Ordering::Acquire)
    }

    /// The most documents ever admitted at once.
    #[cfg(test)]
    pub(crate) fn peak(&self) -> usize {
        self.peak.load(Ordering::Acquire)
    }

    /// How many admissions have been granted.
    #[cfg(test)]
    pub(crate) fn admitted(&self) -> usize {
        self.admitted.load(Ordering::Acquire)
    }

    /// Test seam: an admitted analysis waits for one permit of `hold` before it
    /// starts, so a test can keep the admitted documents in flight; closing
    /// `hold` lets every later analysis straight through.
    #[cfg(test)]
    pub(crate) fn holding(mut self, hold: Arc<Semaphore>) -> Self {
        self.hold = Some(hold);
        self
    }
}

/// One admitted document; dropping it lets the next one in.
pub(crate) struct Admitted {
    _permit: OwnedSemaphorePermit,
    admission: Arc<AnalysisAdmission>,
}

impl Drop for Admitted {
    fn drop(&mut self) {
        self.admission.in_flight.fetch_sub(1, Ordering::AcqRel);
    }
}
