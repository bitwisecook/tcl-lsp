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

//! Campaign runner: generate → run both backends → compare → record.
//!
//! Runs over the native subprocess harness, generalised (issue #1313) to any
//! pair of [`crate::engine::Engine`]s — not just `tclvm` / `tclsh`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::engine::Engine;
use crate::findings::{Category, Finding, FindingContext, Registry};
use crate::generator::{GenConfig, generate};
use crate::harness::{Verdict, compare_outcomes, run_backend, write_script};

/// Aggregate campaign statistics.
#[derive(Debug, Default, Clone, Copy)]
pub struct Stats {
    /// Scripts generated and run.
    pub total: u64,
    /// Matching (no divergence) runs.
    pub matched: u64,
    /// stdout-mismatch findings.
    pub stdout_mismatch: u64,
    /// status-mismatch findings.
    pub status_mismatch: u64,
    /// error-text-mismatch findings (only when error-text comparison is on —
    /// see [`Campaign::compare_error_text`]).
    pub error_text_mismatch: u64,
    /// subject-timeout findings.
    pub timeout: u64,
    /// Runs skipped (reference timeout / backend unavailable).
    pub skipped: u64,
    /// New findings recorded this campaign.
    pub new_findings: u64,
}

impl Stats {
    /// Total findings (any category) this campaign.
    #[must_use]
    pub fn findings(&self) -> u64 {
        self.stdout_mismatch + self.status_mismatch + self.error_text_mismatch + self.timeout
    }
}

/// One engine's resolved invocation: a binary plus fixed leading arguments
/// (see [`crate::engine::Engine::args`]).
#[derive(Debug, Clone)]
pub struct Backend<'a> {
    /// Stable backend identity persisted with each finding.
    pub engine: Engine,
    /// The engine's binary.
    pub binary: &'a Path,
    /// Fixed leading arguments, before the script-file argument.
    pub args: Vec<String>,
}

/// Configuration for one campaign, over any pair of engines (issue #1313 —
/// previously hardcoded to `tclvm` subject / `tclsh` reference).
pub struct Campaign<'a> {
    /// Reference engine (the presumed-correct oracle for this pair).
    pub reference: Backend<'a>,
    /// Subject engine (the implementation under test).
    pub subject: Backend<'a>,
    /// Per-script wall-clock timeout.
    pub timeout: Duration,
    /// Generator tunables.
    pub config: GenConfig,
    /// Findings registry.
    pub registry: &'a Registry,
    /// Scratch directory for generated `.tcl` files.
    pub scratch: PathBuf,
    /// Whether to additionally compare error message text when both engines
    /// error (see `harness::compare_outcomes`). Off by default.
    pub compare_error_text: bool,
    /// Which Tcl release each engine emulates, probed once before the
    /// campaign starts. Stamped onto every finding so a version-skewed pair
    /// can never again masquerade as a bug list (issue #1328 — see
    /// [`crate::version`]).
    pub versions: crate::version::PairVersions,
}

impl Campaign<'_> {
    /// Run `iterations` scripts starting at `base_seed`, invoking `progress`
    /// after each iteration with the running stats.
    pub fn run(
        &self,
        base_seed: u64,
        iterations: u64,
        mut progress: impl FnMut(u64, &Stats),
    ) -> Stats {
        let mut stats = Stats::default();
        for i in 0..iterations {
            let seed = base_seed.wrapping_add(i);
            stats.total += 1;
            self.run_one(seed, &mut stats);
            progress(i + 1, &stats);
        }
        stats
    }

    /// Generate, run, and compare a single seed, updating `stats` and recording
    /// any finding.
    pub fn run_one(&self, seed: u64, stats: &mut Stats) -> Verdict {
        let script = generate(seed, &self.config);
        let Ok(path) = write_script(&self.scratch, seed, &script) else {
            stats.skipped += 1;
            return Verdict::Skipped;
        };
        let reference = run_backend(
            self.reference.binary,
            &self.reference.args,
            &path,
            self.timeout,
        );
        let subject = run_backend(self.subject.binary, &self.subject.args, &path, self.timeout);
        let _ = std::fs::remove_file(&path);

        let verdict = compare_outcomes(&reference, &subject, self.compare_error_text);
        match verdict {
            Verdict::Match => stats.matched += 1,
            Verdict::Skipped => stats.skipped += 1,
            Verdict::StdoutMismatch => stats.stdout_mismatch += 1,
            Verdict::StatusMismatch => stats.status_mismatch += 1,
            Verdict::ErrorTextMismatch => stats.error_text_mismatch += 1,
            Verdict::Timeout => stats.timeout += 1,
        }
        if let Some(category) = Category::from_verdict(verdict) {
            let finding = Finding::new(
                seed,
                category,
                &script,
                &FindingContext {
                    reference: &reference,
                    subject: &subject,
                    reference_engine: self.reference.engine.label(),
                    subject_engine: self.subject.engine.label(),
                    versions: &self.versions,
                },
            );
            if self.registry.record(&finding).unwrap_or(false) {
                stats.new_findings += 1;
            }
        }
        verdict
    }
}
