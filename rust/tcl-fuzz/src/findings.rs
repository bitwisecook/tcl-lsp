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

//! Findings registry: persist and categorise differential divergences.
//!
//! A finding is keyed by its `seed` (so it replays exactly) and saved as a
//! JSON record plus the raw `.tcl` script under the findings directory. The
//! registry de-duplicates by seed and summarises by category.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::harness::{Outcome, Verdict};

/// The category of a finding — the [`Verdict`] that produced it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// stdout differed between engines.
    StdoutMismatch,
    /// Error status differed.
    StatusMismatch,
    /// Both engines errored, but their error message text differed (only
    /// recorded when error-text comparison is enabled — see
    /// `harness::compare_outcomes`).
    ErrorTextMismatch,
    /// The subject hung.
    Timeout,
}

impl Category {
    /// Map a non-`Match`/`Skipped` verdict to a category.
    #[must_use]
    pub fn from_verdict(v: Verdict) -> Option<Self> {
        match v {
            Verdict::StdoutMismatch => Some(Self::StdoutMismatch),
            Verdict::StatusMismatch => Some(Self::StatusMismatch),
            Verdict::ErrorTextMismatch => Some(Self::ErrorTextMismatch),
            Verdict::Timeout => Some(Self::Timeout),
            Verdict::Match | Verdict::Skipped => None,
        }
    }
}

/// A recorded divergence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// The seed that reproduces it.
    pub seed: u64,
    /// What diverged.
    pub category: Category,
    /// The generated script.
    pub script: String,
    /// Reference engine's stdout, when it ran.
    pub reference_stdout: String,
    /// Subject engine's stdout, when it ran.
    pub subject_stdout: String,
    /// Reference error status.
    pub reference_errored: bool,
    /// Subject error status.
    pub subject_errored: bool,
    /// Reference engine's stderr, when it ran. Always captured (independent of
    /// whether error-text comparison drove this finding's category), so a
    /// triager always has both engines' error text to hand.
    #[serde(default)]
    pub reference_stderr: String,
    /// Subject engine's stderr, when it ran.
    #[serde(default)]
    pub subject_stderr: String,
}

impl Finding {
    /// Build a finding from a campaign iteration's outcomes.
    #[must_use]
    pub fn new(
        seed: u64,
        category: Category,
        script: &str,
        reference: &Outcome,
        subject: &Outcome,
    ) -> Self {
        let (reference_stdout, reference_stderr, reference_errored) = unpack(reference);
        let (subject_stdout, subject_stderr, subject_errored) = unpack(subject);
        Self {
            seed,
            category,
            script: script.to_owned(),
            reference_stdout,
            subject_stdout,
            reference_errored,
            subject_errored,
            reference_stderr,
            subject_stderr,
        }
    }
}

fn unpack(o: &Outcome) -> (String, String, bool) {
    match o {
        Outcome::Ran {
            stdout,
            stderr,
            errored,
        } => (stdout.clone(), stderr.clone(), *errored),
        Outcome::Timeout => ("<timeout>".to_owned(), String::new(), true),
        Outcome::Unavailable(m) => (format!("<unavailable: {m}>"), String::new(), true),
    }
}

/// On-disk findings registry rooted at a directory.
pub struct Registry {
    dir: PathBuf,
}

impl Registry {
    /// Open (creating if needed) a registry at `dir`.
    ///
    /// # Errors
    /// If the directory cannot be created.
    pub fn open(dir: impl Into<PathBuf>) -> std::io::Result<Self> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    /// Persist a finding (JSON record + raw `.tcl`), keyed by seed. Returns
    /// `true` if it was new, `false` if a finding for that seed already exists.
    ///
    /// # Errors
    /// On any filesystem write failure.
    pub fn record(&self, finding: &Finding) -> std::io::Result<bool> {
        let json = self.dir.join(format!("finding-{}.json", finding.seed));
        if json.exists() {
            return Ok(false);
        }
        std::fs::write(
            self.dir.join(format!("finding-{}.tcl", finding.seed)),
            &finding.script,
        )?;
        std::fs::write(
            &json,
            serde_json::to_string_pretty(finding).unwrap_or_default(),
        )?;
        Ok(true)
    }

    /// Count of recorded findings per category.
    #[must_use]
    pub fn summary(&self) -> BTreeMap<Category, usize> {
        let mut counts: BTreeMap<Category, usize> = BTreeMap::new();
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return counts;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(&path)
                && let Ok(f) = serde_json::from_str::<Finding>(&text)
            {
                *counts.entry(f.category).or_default() += 1;
            }
        }
        counts
    }

    /// Load a previously-recorded finding by seed, if present.
    #[must_use]
    pub fn load(&self, seed: u64) -> Option<Finding> {
        let text = std::fs::read_to_string(self.dir.join(format!("finding-{seed}.json"))).ok()?;
        serde_json::from_str(&text).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        let p = std::env::temp_dir().join(format!("tcl-fuzz-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn record_dedups_and_summarises() {
        let dir = tmp();
        let reg = Registry::open(&dir).unwrap();
        let f = Finding::new(
            7,
            Category::StdoutMismatch,
            "puts 1",
            &Outcome::Ran {
                stdout: "1\n".into(),
                stderr: String::new(),
                errored: false,
            },
            &Outcome::Ran {
                stdout: "2\n".into(),
                stderr: String::new(),
                errored: false,
            },
        );
        assert!(reg.record(&f).unwrap());
        assert!(
            !reg.record(&f).unwrap(),
            "second record of same seed is a no-op"
        );
        assert_eq!(reg.summary().get(&Category::StdoutMismatch), Some(&1));
        assert_eq!(reg.load(7).unwrap().subject_stdout, "2\n");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
