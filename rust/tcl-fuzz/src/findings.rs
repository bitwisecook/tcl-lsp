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
    /// Stable reference backend identity (`tclsh`, `tclvm`, or
    /// `runtime-rust`).
    #[serde(default)]
    pub reference_engine: String,
    /// Stable subject backend identity.
    #[serde(default)]
    pub subject_engine: String,
    /// The Tcl release the **reference** engine reported
    /// (`[info patchlevel]`), when it could be probed. A divergence is evidence
    /// of a bug only when both engines speak the same version of the language.
    #[serde(default)]
    pub reference_version: Option<String>,
    /// Subject `[info patchlevel]`, when probing succeeded.
    #[serde(default)]
    pub subject_version: Option<String>,
    /// Whether the two engines emulate different Tcl release *lines*. `true`
    /// makes this finding suspect: read it as "the engines disagree" only
    /// after ruling out "the versions disagree".
    #[serde(default)]
    pub version_skew: bool,
}

/// The backend outcomes and identities attached to one finding.
///
/// Keeping this execution context together prevents callers from accidentally
/// swapping one outcome while leaving its engine identity unchanged.
pub struct FindingContext<'a> {
    /// Reference backend outcome.
    pub reference: &'a Outcome,
    /// Subject backend outcome.
    pub subject: &'a Outcome,
    /// Stable reference backend identity.
    pub reference_engine: &'a str,
    /// Stable subject backend identity.
    pub subject_engine: &'a str,
    /// Backend versions probed for this campaign.
    pub versions: &'a crate::version::PairVersions,
}

impl Finding {
    /// Build a finding from a campaign iteration's outcomes.
    #[must_use]
    pub fn new(seed: u64, category: Category, script: &str, context: &FindingContext<'_>) -> Self {
        let (reference_stdout, reference_stderr, reference_errored) = unpack(context.reference);
        let (subject_stdout, subject_stderr, subject_errored) = unpack(context.subject);
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
            reference_engine: context.reference_engine.to_owned(),
            subject_engine: context.subject_engine.to_owned(),
            reference_version: context
                .versions
                .reference
                .as_ref()
                .map(|version| version.patchlevel.clone()),
            subject_version: context
                .versions
                .subject
                .as_ref()
                .map(|version| version.patchlevel.clone()),
            version_skew: context.versions.skewed(),
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

/// Directory-backed persistence for single-file reproducers from fuzzer arms
/// that do not have a paired-engine [`Registry`] finding.
pub struct ReproducerStore {
    dir: PathBuf,
}

impl ReproducerStore {
    /// Open (creating if needed) the directory for standalone reproducers.
    ///
    /// # Errors
    /// If the directory cannot be created.
    pub fn open(dir: impl Into<PathBuf>) -> std::io::Result<Self> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    /// Persist one reproducer under its stable file name.
    ///
    /// # Errors
    /// If the reproducer cannot be written.
    pub fn write(&self, file_name: &str, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
        std::fs::write(self.dir.join(file_name), contents)
    }
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

    /// A registry directory unique to `name` as well as the process, so tests
    /// running in parallel never share (or wipe) one another's registry.
    fn tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("tcl-fuzz-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn record_dedups_and_summarises() {
        let dir = tmp("dedup");
        let reg = Registry::open(&dir).unwrap();
        let f = Finding::new(
            7,
            Category::StdoutMismatch,
            "puts 1",
            &FindingContext {
                reference: &Outcome::Ran {
                    stdout: "1\n".into(),
                    stderr: String::new(),
                    errored: false,
                },
                subject: &Outcome::Ran {
                    stdout: "2\n".into(),
                    stderr: String::new(),
                    errored: false,
                },
                reference_engine: "tclsh",
                subject_engine: "tclvm",
                versions: &crate::version::PairVersions::default(),
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

    /// Every finding carries the releases it was produced against, and they
    /// survive the round trip to disk — the whole point of issue #1328's
    /// harness change: a version-skewed campaign must be recognisable from
    /// the registry alone, long after the run.
    #[test]
    fn a_finding_records_both_engines_versions_and_the_skew_flag() {
        let dir = tmp("versions");
        let reg = Registry::open(&dir).unwrap();
        let ran = |s: &str| Outcome::Ran {
            stdout: s.to_owned(),
            stderr: String::new(),
            errored: false,
        };
        let f = Finding::new(
            11,
            Category::StatusMismatch,
            "namespace eval n { append i baz }",
            &FindingContext {
                reference: &ran("a\n"),
                subject: &ran("b\n"),
                reference_engine: "tclsh",
                subject_engine: "runtime-rust",
                versions: &crate::version::PairVersions {
                    reference: Some(crate::version::EngineVersion::parse("8.6.16")),
                    subject: Some(crate::version::EngineVersion::parse("9.0.4")),
                },
            },
        );
        assert!(reg.record(&f).unwrap());
        let back = reg.load(11).unwrap();
        assert_eq!(back.reference_engine, "tclsh");
        assert_eq!(back.subject_engine, "runtime-rust");
        assert_eq!(back.reference_version.as_deref(), Some("8.6.16"));
        assert_eq!(back.subject_version.as_deref(), Some("9.0.4"));
        assert!(back.version_skew, "8.6 oracle vs 9.0 subject is skew");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reproducer_store_creates_its_directory_and_persists_a_script() {
        let dir = tmp("reproducer-success");
        let store = ReproducerStore::open(&dir).expect("creates findings directory");
        store
            .write("seed-7.tcl", "puts reproduced")
            .expect("writes reproducer");
        assert_eq!(
            std::fs::read_to_string(dir.join("seed-7.tcl")).unwrap(),
            "puts reproduced"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reproducer_store_surfaces_directory_and_write_failures() {
        let parent_file = tmp("reproducer-parent-file");
        std::fs::write(&parent_file, "not a directory").unwrap();
        assert!(ReproducerStore::open(parent_file.join("findings")).is_err());
        let _ = std::fs::remove_file(&parent_file);

        let dir = tmp("reproducer-write-error");
        let store = ReproducerStore::open(&dir).unwrap();
        std::fs::create_dir(dir.join("not-a-file")).unwrap();
        assert!(store.write("not-a-file", "puts lost").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
