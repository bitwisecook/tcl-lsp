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

//! Sampled private Rust timings for focused constructor diagnostics.

use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

const MAX_LINES: usize = 1024;
static LINES: AtomicUsize = AtomicUsize::new(0);
static HOST: AtomicUsize = AtomicUsize::new(0);
static BOOTSTRAP: AtomicUsize = AtomicUsize::new(0);
static IDENTITIES: AtomicUsize = AtomicUsize::new(0);
static PIN: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static APPEND_SCOPE: Cell<bool> = const { Cell::new(false) };
}

#[derive(Clone, Copy, Default)]
struct WorkTotals {
    prune_calls: u64,
    prune_ns: u128,
    currency_calls: u64,
    currency_ns: u128,
}

thread_local! {
    static SAMPLED_SCOPES: Cell<usize> = const { Cell::new(0) };
    static WORK_TOTALS: Cell<WorkTotals> = const { Cell::new(WorkTotals {
        prune_calls: 0,
        prune_ns: 0,
        currency_calls: 0,
        currency_ns: 0,
    }) };
}

pub(super) enum Work {
    Prune,
    Currency,
}

pub(super) struct WorkMeasurement {
    kind: Work,
    started: Option<Instant>,
}

impl WorkMeasurement {
    pub(super) fn new(kind: Work) -> Self {
        Self {
            kind,
            started: (SAMPLED_SCOPES.get() != 0).then(Instant::now),
        }
    }
}

impl Drop for WorkMeasurement {
    fn drop(&mut self) {
        let Some(started) = self.started else {
            return;
        };
        let elapsed = started.elapsed().as_nanos();
        let mut totals = WORK_TOTALS.get();
        match self.kind {
            Work::Prune => {
                totals.prune_calls = totals.prune_calls.saturating_add(1);
                totals.prune_ns = totals.prune_ns.saturating_add(elapsed);
            }
            Work::Currency => {
                totals.currency_calls = totals.currency_calls.saturating_add(1);
                totals.currency_ns = totals.currency_ns.saturating_add(elapsed);
            }
        }
        WORK_TOTALS.set(totals);
    }
}

pub(crate) struct AppendScope(bool);

impl AppendScope {
    pub(crate) fn enter() -> Self {
        let enabled = std::env::var("TCL_LSP_RUNTIME_CONSTRUCTOR_DIAGNOSTIC").as_deref() == Ok("1");
        Self(APPEND_SCOPE.replace(enabled))
    }
}

impl Drop for AppendScope {
    fn drop(&mut self) {
        APPEND_SCOPE.set(self.0);
    }
}

pub(super) enum Kind {
    Host,
    Bootstrap,
    Identities,
    Pin,
}

impl Kind {
    fn counter(&self) -> &'static AtomicUsize {
        match self {
            Self::Host => &HOST,
            Self::Bootstrap => &BOOTSTRAP,
            Self::Identities => &IDENTITIES,
            Self::Pin => &PIN,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Host => "host",
            Self::Bootstrap => "bootstrap",
            Self::Identities => "identities",
            Self::Pin => "pin",
        }
    }
}

pub(super) struct ConstructorDiagnostics {
    kind: Kind,
    ordinal: usize,
    started: Option<Instant>,
    previous: Option<Instant>,
    work_started: WorkTotals,
}

impl ConstructorDiagnostics {
    pub(super) fn new(kind: Kind) -> Self {
        let enabled = APPEND_SCOPE.get();
        let ordinal = if enabled {
            kind.counter().fetch_add(1, Ordering::Relaxed)
        } else {
            0
        };
        let selected = enabled && (ordinal < 8 || ordinal.is_multiple_of(32));
        if selected {
            SAMPLED_SCOPES.set(SAMPLED_SCOPES.get().saturating_add(1));
        }
        let started = selected.then(Instant::now);
        let mut diagnostic = Self {
            kind,
            ordinal,
            started,
            previous: started,
            work_started: if selected {
                WORK_TOTALS.get()
            } else {
                WorkTotals::default()
            },
        };
        diagnostic.stage("start");
        diagnostic
    }

    pub(super) fn stage(&mut self, stage: &'static str) {
        let Some(started) = self.started else {
            return;
        };
        let line = LINES.fetch_add(1, Ordering::Relaxed);
        if line >= MAX_LINES {
            if line == MAX_LINES {
                eprintln!("CTOR_DIAGNOSTIC_TRUNCATED max_lines={MAX_LINES}");
            }
            return;
        }
        let now = Instant::now();
        let previous = self.previous.replace(now).unwrap_or(started);
        let work = WORK_TOTALS.get();
        eprintln!(
            "CTOR_DIAGNOSTIC kind={} sample={} stage={} total_us={} delta_us={} prune_calls={} prune_ns={} currency_calls={} currency_ns={}",
            self.kind.label(),
            self.ordinal,
            stage,
            now.duration_since(started).as_micros(),
            now.duration_since(previous).as_micros(),
            work.prune_calls.saturating_sub(self.work_started.prune_calls),
            work.prune_ns.saturating_sub(self.work_started.prune_ns),
            work.currency_calls.saturating_sub(self.work_started.currency_calls),
            work.currency_ns.saturating_sub(self.work_started.currency_ns),
        );
    }
}

impl Drop for ConstructorDiagnostics {
    fn drop(&mut self) {
        if self.started.is_some() {
            SAMPLED_SCOPES.set(SAMPLED_SCOPES.get().saturating_sub(1));
        }
    }
}
