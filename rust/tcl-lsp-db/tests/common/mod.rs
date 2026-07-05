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

//! Shared corpus/fuzz test helpers.
//!
//! [`Progress`] gives long corpus sweeps **durable, flushed progress**: one line
//! per completed file (a "chunk") and one per finding, written *and flushed* to a
//! log file as they happen and mirrored to stderr. So a sweep can be watched live
//! (`tail -f target/fuzz-progress/<name>.log`), and a `SIGKILL` mid-run preserves
//! every completed chunk's result plus every finding so far — instead of losing
//! everything because the test only asserts at the very end.
//!
//! Runs are also **resumable**: [`Progress::skip`] reads `$TCL_FUZZ_SKIP` so a
//! killed sweep can restart roughly where it stopped, and [`Progress::limit`]
//! reads `$TCL_FUZZ_LIMIT` to cap a chunk to a bounded slice.
#![allow(dead_code)]

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

/// Durable, flushed progress log for one corpus sweep.
pub struct Progress {
    file: Option<File>,
    start: Instant,
    findings: usize,
}

impl Progress {
    /// Open (truncate) `$TCL_FUZZ_PROGRESS_DIR/<name>.log` (default
    /// `<repo>/target/fuzz-progress/<name>.log`) and write a START line.
    pub fn new(name: &str) -> Self {
        let dir = std::env::var("TCL_FUZZ_PROGRESS_DIR").map_or_else(
            |_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/fuzz-progress"),
            PathBuf::from,
        );
        let _ = fs::create_dir_all(&dir);
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(dir.join(format!("{name}.log")))
            .ok();
        let mut p = Self {
            file,
            start: Instant::now(),
            findings: 0,
        };
        p.line(&format!("START {name} (skip={}, limit={:?})", Self::skip(), Self::limit()));
        p
    }

    fn line(&mut self, msg: &str) {
        let out = format!("[{:>6}s] {msg}", self.start.elapsed().as_secs());
        eprintln!("{out}");
        if let Some(f) = self.file.as_mut() {
            let _ = writeln!(f, "{out}");
            let _ = f.flush();
        }
    }

    /// Record one completed chunk (usually a file). Flushed immediately.
    pub fn tick(&mut self, done: usize, total: usize, detail: &str) {
        self.line(&format!(
            "{done}/{total} findings={} {detail}",
            self.findings
        ));
    }

    /// Record a divergence/finding — written and flushed at once, so a kill can
    /// never lose it. Returns the running finding count.
    pub fn finding(&mut self, msg: &str) -> usize {
        self.findings += 1;
        self.line(&format!("FINDING #{}: {msg}", self.findings));
        self.findings
    }

    pub fn findings(&self) -> usize {
        self.findings
    }

    /// Final summary line.
    pub fn finish(&mut self, summary: &str) {
        self.line(&format!("DONE {summary} total_findings={}", self.findings));
    }

    /// Starting file index from `$TCL_FUZZ_SKIP` (default 0) — resume a killed
    /// sweep roughly where it stopped.
    pub fn skip() -> usize {
        std::env::var("TCL_FUZZ_SKIP")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }

    /// Optional cap on files processed from `$TCL_FUZZ_LIMIT` — bound a chunk.
    pub fn limit() -> Option<usize> {
        std::env::var("TCL_FUZZ_LIMIT")
            .ok()
            .and_then(|s| s.parse().ok())
    }

    /// Apply `skip`/`limit` to a gathered file list, returning the slice this
    /// chunk should process along with `(absolute_start_index, total_len)`.
    pub fn slice<T: Clone>(files: &[T]) -> (Vec<T>, usize, usize) {
        let total = files.len();
        let start = Self::skip().min(total);
        let end = Self::limit().map_or(total, |n| (start + n).min(total));
        (files[start..end].to_vec(), start, total)
    }
}
