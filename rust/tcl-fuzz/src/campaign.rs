//! Campaign runner: generate → run both backends → compare → record.
//!
//! Runs over the native `tclvm` / `tclsh` subprocess harness.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::findings::{Category, Finding, Registry};
use crate::generator::{GenConfig, generate};
use crate::harness::{Verdict, compare, run_backend, write_script};

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
        self.stdout_mismatch + self.status_mismatch + self.timeout
    }
}

/// Configuration for one campaign.
pub struct Campaign<'a> {
    /// Reference engine (`tclsh`).
    pub tclsh: &'a Path,
    /// Subject engine (`tclvm`).
    pub tclvm: &'a Path,
    /// Per-script wall-clock timeout.
    pub timeout: Duration,
    /// Generator tunables.
    pub config: GenConfig,
    /// Findings registry.
    pub registry: &'a Registry,
    /// Scratch directory for generated `.tcl` files.
    pub scratch: PathBuf,
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
        let reference = run_backend(self.tclsh, &path, self.timeout);
        let subject = run_backend(self.tclvm, &path, self.timeout);
        let _ = std::fs::remove_file(&path);

        let verdict = compare(&reference, &subject);
        match verdict {
            Verdict::Match => stats.matched += 1,
            Verdict::Skipped => stats.skipped += 1,
            Verdict::StdoutMismatch => stats.stdout_mismatch += 1,
            Verdict::StatusMismatch => stats.status_mismatch += 1,
            Verdict::Timeout => stats.timeout += 1,
        }
        if let Some(category) = Category::from_verdict(verdict) {
            let finding = Finding::new(seed, category, &script, &reference, &subject);
            if self.registry.record(&finding).unwrap_or(false) {
                stats.new_findings += 1;
            }
        }
        verdict
    }
}
