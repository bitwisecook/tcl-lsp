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

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

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
    /// The Tcl release explicitly selected for the whole campaign. Unlike the
    /// observed patchlevels below, this is the configuration replay must
    /// restore before it can claim to reproduce a finding.
    #[serde(default)]
    pub tcl_version: Option<String>,
    /// Schema marker proving this record was written after replay gained a
    /// campaign-level configuration. Its presence makes an explicitly
    /// unpinned campaign (`tcl_version: null`) distinguishable from an older
    /// record that simply lacks the field.
    #[serde(default)]
    pub replay_metadata_version: Option<u8>,
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
            tcl_version: context
                .versions
                .tcl_version
                .map(tcl_dialect::TclVersion::version_string)
                .map(str::to_owned),
            replay_metadata_version: Some(1),
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

    /// Return this finding's configured Tcl release, if its campaign was
    /// deliberately unpinned.
    ///
    /// A valid explicit release is independently replay-safe, including older
    /// records written before the schema marker existed. A release-less record
    /// is replayable only when the marker proves it came from a deliberate
    /// unpinned campaign; otherwise the old subject-only path left no way to
    /// establish its original configuration.
    pub fn campaign_tcl_version(&self) -> Result<Option<tcl_dialect::TclVersion>, String> {
        if let Some(raw) = self.tcl_version.as_deref() {
            return tcl_dialect::TclVersion::from_package_version(raw)
                .map(Some)
                .ok_or_else(|| {
                    format!(
                        "finding {} has an unrecognised recorded Tcl release {raw:?}",
                        self.seed,
                    )
                });
        }
        if self.replay_metadata_version == Some(1) {
            Ok(None)
        } else {
            Err(format!(
                "finding {} predates replay-safe campaign metadata; it cannot be replayed safely",
                self.seed,
            ))
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

/// The four files which make up one on-disk finding transaction.
///
/// The final `.tcl` and `.json` files are the public record.  Their `.stage`
/// counterparts are intentionally retained after an interrupted write: a
/// later replay must report damage rather than mistake the seed for an
/// arbitrary, unrecorded input.
struct RecordPaths {
    tcl: PathBuf,
    json: PathBuf,
    tcl_stage: PathBuf,
    json_stage: PathBuf,
}

/// Whether a record's complete pair is present, absent, or visibly damaged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecordState {
    Absent,
    Complete,
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
        self.record_inner(finding, |_| {})
    }

    /// Write one finding after invoking `before_promotion` once both staging
    /// files are durable and before either final name is claimed.
    fn record_inner(
        &self,
        finding: &Finding,
        before_promotion: impl FnOnce(&RecordPaths),
    ) -> std::io::Result<bool> {
        let paths = self.paths(finding.seed);
        if Self::record_state(finding.seed, &paths)? == RecordState::Complete {
            return Ok(false);
        }

        let json = serde_json::to_vec_pretty(finding).map_err(io::Error::other)?;
        write_new_and_sync(&paths.tcl_stage, finding.script.as_bytes())?;
        write_new_and_sync(&paths.json_stage, &json)?;
        sync_directory(&self.dir)?;
        before_promotion(&paths);

        // `rename` would atomically *replace* an unexpectedly-created final
        // file.  `create_new` claims each final name without replacement;
        // copying the already-synced stage then flushing it means an error or
        // interruption leaves a final/stage combination that replay rejects.
        // Removing a stage only after its final file is durable preserves that
        // fail-closed evidence.
        promote_no_replace(&paths.tcl_stage, &paths.tcl)?;
        promote_no_replace(&paths.json_stage, &paths.json)?;
        sync_directory(&self.dir)?;
        Ok(true)
    }

    #[cfg(test)]
    fn record_with_interleaving(
        &self,
        finding: &Finding,
        before_promotion: impl FnOnce(&RecordPaths),
    ) -> std::io::Result<bool> {
        self.record_inner(finding, before_promotion)
    }

    /// Load a previously-recorded finding by seed, if present.
    #[cfg(test)]
    #[must_use]
    pub fn load(&self, seed: u64) -> Option<Finding> {
        self.load_checked(seed).ok().flatten()
    }

    /// Count of recorded findings per category.
    #[must_use = "inspect the summary result"]
    pub fn summary(&self) -> io::Result<BTreeMap<Category, usize>> {
        let mut counts: BTreeMap<Category, usize> = BTreeMap::new();
        let mut seeds = BTreeSet::new();
        for entry in std::fs::read_dir(&self.dir)? {
            let path = entry?.path();
            if let Some(seed) = record_seed_from_path(&path) {
                seeds.insert(seed);
            }
        }

        for seed in seeds {
            let paths = self.paths(seed);
            if Self::record_state(seed, &paths)? != RecordState::Complete {
                continue;
            }
            let text = std::fs::read_to_string(&paths.json)?;
            let finding = serde_json::from_str::<Finding>(&text).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{}: {error}", paths.json.display()),
                )
            })?;
            if finding.seed != seed {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{} records seed {}, expected {seed}",
                        paths.json.display(),
                        finding.seed
                    ),
                ));
            }
            *counts.entry(finding.category).or_default() += 1;
        }
        Ok(counts)
    }

    /// Load a finding while distinguishing an absent record from a damaged
    /// record. Replay must fail closed when either companion is missing, a
    /// staged transaction remains, or the JSON cannot be read or decoded.
    pub fn load_checked(&self, seed: u64) -> std::io::Result<Option<Finding>> {
        let paths = self.paths(seed);
        match Self::record_state(seed, &paths)? {
            RecordState::Absent => return Ok(None),
            RecordState::Complete => {}
        }
        let path = paths.json;
        match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).map(Some).map_err(|error| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{}: {error}", path.display()),
                )
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn paths(&self, seed: u64) -> RecordPaths {
        let stem = self.dir.join(format!("finding-{seed}"));
        RecordPaths {
            tcl: stem.with_extension("tcl"),
            json: stem.with_extension("json"),
            tcl_stage: stem.with_extension("tcl.stage"),
            json_stage: stem.with_extension("json.stage"),
        }
    }

    /// Verify that a seed is represented by exactly the final pair or by no
    /// files at all.  Treat every other state as corruption.  This includes
    /// a fully staged pair: its promotion may have been interrupted, and it
    /// must not be mistaken for an absent, ad-hoc seed.
    fn record_state(seed: u64, paths: &RecordPaths) -> io::Result<RecordState> {
        let present = [
            ("Tcl", &paths.tcl),
            ("JSON", &paths.json),
            ("staged Tcl", &paths.tcl_stage),
            ("staged JSON", &paths.json_stage),
        ]
        .into_iter()
        .map(|(kind, path)| path_exists(path).map(|exists| (kind, path, exists)))
        .collect::<io::Result<Vec<_>>>()?;
        let final_tcl = present[0].2;
        let final_json = present[1].2;
        let staged_tcl = present[2].2;
        let staged_json = present[3].2;

        if final_tcl && final_json && !staged_tcl && !staged_json {
            return Ok(RecordState::Complete);
        }
        if !final_tcl && !final_json && !staged_tcl && !staged_json {
            return Ok(RecordState::Absent);
        }
        let files = present
            .iter()
            .filter(|(_, _, exists)| *exists)
            .map(|(_, path, _)| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "finding {seed} has an incomplete or interrupted paired record ({files}); repair or remove every companion before replaying"
            ),
        ))
    }
}

fn path_exists(path: &Path) -> io::Result<bool> {
    match std::fs::metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

/// Extract a finding seed from any of the four names in a record transaction.
/// Scanning staged names as well as final names lets `summary` reject a
/// transaction that has no final JSON left to enumerate.
fn record_seed_from_path(path: &Path) -> Option<u64> {
    let name = path.file_name()?.to_str()?;
    let stem = name.strip_prefix("finding-")?;
    let raw = stem
        .strip_suffix(".json.stage")
        .or_else(|| stem.strip_suffix(".tcl.stage"))
        .or_else(|| stem.strip_suffix(".json"))
        .or_else(|| stem.strip_suffix(".tcl"))?;
    raw.parse().ok()
}

fn write_new_and_sync(path: &Path, contents: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(contents)?;
    file.sync_all()
}

/// Copy `stage` to a newly claimed `final_path` without replacing an existing
/// final companion, then remove the staging name.
fn promote_no_replace(stage: &Path, final_path: &Path) -> io::Result<()> {
    let mut staged = File::open(stage)?;
    let mut final_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(final_path)?;
    io::copy(&mut staged, &mut final_file)?;
    final_file.sync_all()?;
    drop(final_file);
    if let Some(parent) = final_path.parent() {
        // The final directory entry must be durable before its stage name is
        // removed.  Otherwise a crash can lose both names and make replay
        // mistake this transaction for an absent, ad-hoc seed.
        sync_directory(parent)?;
    }
    std::fs::remove_file(stage)
}

#[cfg(unix)]
fn sync_directory(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

// Windows does not expose POSIX directory fsync through `std::fs::File`.
// Individual staging files are still flushed before their renames, and any
// crash-visible partial promotion is rejected by `record_state`.
#[cfg(not(unix))]
fn sync_directory(_dir: &Path) -> io::Result<()> {
    Ok(())
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

    fn test_finding(seed: u64) -> Finding {
        let reference = Outcome::Ran {
            stdout: "reference\n".to_owned(),
            stderr: String::new(),
            errored: false,
        };
        let subject = Outcome::Ran {
            stdout: "subject\n".to_owned(),
            stderr: String::new(),
            errored: false,
        };
        Finding::new(
            seed,
            Category::StdoutMismatch,
            "puts paired",
            &FindingContext {
                reference: &reference,
                subject: &subject,
                reference_engine: "tclsh",
                subject_engine: "tclvm",
                versions: &crate::version::PairVersions::default(),
            },
        )
    }

    fn assert_damaged(registry: &Registry, seed: u64) {
        let error = registry
            .load_checked(seed)
            .expect_err("an incomplete record must never look absent");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("incomplete or interrupted"));
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
        assert_eq!(
            reg.summary().unwrap().get(&Category::StdoutMismatch),
            Some(&1)
        );
        assert_eq!(reg.load(7).unwrap().subject_stdout, "2\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_checked_rejects_a_tcl_only_orphan() {
        let dir = tmp("tcl-only-orphan");
        let registry = Registry::open(&dir).unwrap();
        std::fs::write(dir.join("finding-23.tcl"), "puts orphan").unwrap();

        assert_damaged(&registry, 23);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_checked_rejects_a_json_only_orphan() {
        let dir = tmp("json-only-orphan");
        let registry = Registry::open(&dir).unwrap();
        let finding = test_finding(29);
        std::fs::write(
            dir.join("finding-29.json"),
            serde_json::to_vec_pretty(&finding).unwrap(),
        )
        .unwrap();

        assert_damaged(&registry, 29);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_checked_rejects_an_interrupted_staged_record() {
        let dir = tmp("staged-record");
        let registry = Registry::open(&dir).unwrap();
        let finding = test_finding(31);
        std::fs::write(dir.join("finding-31.tcl.stage"), &finding.script).unwrap();
        std::fs::write(
            dir.join("finding-31.json.stage"),
            serde_json::to_vec_pretty(&finding).unwrap(),
        )
        .unwrap();

        assert_damaged(&registry, 31);
        assert!(
            registry.record(&finding).is_err(),
            "do not overwrite damage"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn record_promotes_a_complete_pair_without_leaving_stages() {
        let dir = tmp("atomic-promotion");
        let registry = Registry::open(&dir).unwrap();
        let finding = test_finding(41);

        assert!(registry.record(&finding).unwrap());
        assert_eq!(
            std::fs::read_to_string(dir.join("finding-41.tcl")).unwrap(),
            finding.script
        );
        assert!(dir.join("finding-41.json").exists());
        assert!(!dir.join("finding-41.tcl.stage").exists());
        assert!(!dir.join("finding-41.json.stage").exists());
        assert_eq!(registry.load_checked(41).unwrap().unwrap().seed, 41);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn promotion_never_overwrites_an_interleaved_final_and_keeps_damage_visible() {
        let dir = tmp("no-replace-interleaving");
        let registry = Registry::open(&dir).unwrap();
        let finding = test_finding(43);

        let error = registry
            .record_with_interleaving(&finding, |paths| {
                // Deterministically model another writer creating a final
                // companion after `record_state`'s precheck but before this
                // transaction promotes either staging file.
                std::fs::write(&paths.tcl, "puts external writer").unwrap();
            })
            .expect_err("promotion must not replace a concurrent final");

        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(
            std::fs::read_to_string(dir.join("finding-43.tcl")).unwrap(),
            "puts external writer"
        );
        // Keep the stages rather than deleting evidence of the interrupted
        // transaction.  `load_checked` then makes the damage fail closed.
        assert!(dir.join("finding-43.tcl.stage").exists());
        assert!(dir.join("finding-43.json.stage").exists());
        assert_damaged(&registry, 43);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn summary_rejects_a_staged_companion() {
        let dir = tmp("summary-staged-companion");
        let registry = Registry::open(&dir).unwrap();
        let finding = test_finding(47);
        assert!(registry.record(&finding).unwrap());
        std::fs::write(dir.join("finding-47.json.stage"), b"staged").unwrap();

        let error = registry
            .summary()
            .expect_err("summary must fail closed on an interrupted transaction");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("incomplete or interrupted"));
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
                    tcl_version: None,
                    reference: Some(crate::version::EngineVersion::parse("8.6.16")),
                    subject: Some(crate::version::EngineVersion::parse("9.0.4")),
                },
            },
        );
        assert!(reg.record(&f).unwrap());
        let back = reg.load(11).unwrap();
        assert_eq!(back.reference_engine, "tclsh");
        assert_eq!(back.subject_engine, "runtime-rust");
        assert_eq!(back.tcl_version, None);
        assert_eq!(back.reference_version.as_deref(), Some("8.6.16"));
        assert_eq!(back.subject_version.as_deref(), Some("9.0.4"));
        assert!(back.version_skew, "8.6 oracle vs 9.0 subject is skew");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_finding_persists_the_canonical_campaign_release_for_replay() {
        let dir = tmp("campaign-release");
        let reg = Registry::open(&dir).unwrap();
        let outcome = Outcome::Ran {
            stdout: "same\n".to_owned(),
            stderr: String::new(),
            errored: false,
        };
        let finding = Finding::new(
            17,
            Category::StdoutMismatch,
            "puts same",
            &FindingContext {
                reference: &outcome,
                subject: &outcome,
                reference_engine: "runtime-rust",
                subject_engine: "tclvm",
                versions: &crate::version::PairVersions {
                    tcl_version: Some(tcl_dialect::TclVersion::V8_6),
                    reference: Some(crate::version::EngineVersion::parse("8.6.16")),
                    subject: Some(crate::version::EngineVersion::parse("8.6.16")),
                },
            },
        );
        assert!(reg.record(&finding).unwrap());
        let back = reg.load(17).unwrap();
        assert_eq!(back.tcl_version.as_deref(), Some("8.6"));
        assert_eq!(
            back.campaign_tcl_version(),
            Ok(Some(tcl_dialect::TclVersion::V8_6)),
        );
        assert_eq!(back.replay_metadata_version, Some(1));
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
