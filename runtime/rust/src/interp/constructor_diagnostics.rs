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
        let started = selected.then(Instant::now);
        let mut diagnostic = Self {
            kind,
            ordinal,
            started,
            previous: started,
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
        eprintln!(
            "CTOR_DIAGNOSTIC kind={} sample={} stage={} total_us={} delta_us={}",
            self.kind.label(),
            self.ordinal,
            stage,
            now.duration_since(started).as_micros(),
            now.duration_since(previous).as_micros()
        );
    }
}
