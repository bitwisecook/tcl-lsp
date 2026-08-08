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

//! Which Tcl release each engine in a differential pair emulates.
//!
//! # Why this exists
//!
//! Issue #1328 was filed as two `runtime/rust` divergences from C Tcl. It was
//! run against `tclsh8.6` because no 9.0 build was to hand. Re-run against
//! 9.0.4, **all eight** of that campaign's findings disappear: every one was a
//! deliberate 8.6-vs-9.0 semantic change (TIP 461's `lt`/`gt`/`le`/`ge`
//! operators, TIP 521's `isfinite()`/`isinf()`/`isnan()`, and the TIP 278
//! removal of the namespace-scope global fallback for relative variable
//! names). The subject was correct throughout; the *oracle* was a different
//! language version.
//!
//! Nothing in the harness recorded which `tclsh` produced a finding, so
//! nothing could have caught that — the mistake was invisible in the registry
//! and had to be re-derived by hand, twice. This module makes the reference's
//! release a first-class, recorded property of every campaign and every
//! finding, so a version-skewed pair announces itself instead of manufacturing
//! a bug list.
//!
//! # What it does not do
//!
//! It does not decide whether a *particular* divergence is explained by the
//! skew — that needs a per-behaviour model of every cross-version change, which
//! is a much larger piece of work (tracked separately). It records the fact and
//! warns loudly, which is what would have prevented #1328.

use std::path::Path;
use std::time::Duration;

use tcl_dialect::TclVersion;

/// One engine's self-reported release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineVersion {
    /// The raw `[info patchlevel]` string (`"9.0.4"`, `"8.6.16"`).
    pub patchlevel: String,
    /// The release *line* it maps to, when this project models that line.
    /// `None` for a patchlevel outside the modelled 8.4–9.1 range, which is
    /// reported verbatim rather than guessed at.
    pub line: Option<TclVersion>,
}

impl EngineVersion {
    /// Parse a raw `[info patchlevel]` string.
    #[must_use]
    pub fn parse(patchlevel: &str) -> Self {
        let patchlevel = patchlevel.trim().to_owned();
        // `from_package_version` reads major.minor and ignores any patch
        // component, which is exactly the granularity semantics key off.
        let line = TclVersion::from_package_version(&patchlevel);
        Self { patchlevel, line }
    }

    /// Ask `binary` which release it is by running a one-line script through
    /// the same subprocess path the campaign uses.
    ///
    /// `None` when the engine could not be run or printed nothing usable — a
    /// probe failure must never abort a campaign, only leave the version
    /// unrecorded.
    #[must_use]
    pub fn probe(binary: &Path, args: &[String], timeout: Duration) -> Option<Self> {
        let dir = std::env::temp_dir().join(format!("tcl-fuzz-vprobe-{}", std::process::id()));
        std::fs::create_dir_all(&dir).ok()?;
        let script = dir.join("version.tcl");
        std::fs::write(&script, "puts [info patchlevel]\n").ok()?;
        let out = crate::harness::run_backend(binary, args, &script, timeout);
        let _ = std::fs::remove_file(&script);
        let _ = std::fs::remove_dir(&dir);
        let crate::harness::Outcome::Ran { stdout, .. } = out else {
            return None;
        };
        let line = stdout.lines().next()?.trim();
        (!line.is_empty()).then(|| Self::parse(line))
    }
}

/// Both engines' releases, and whether they agree.
#[derive(Debug, Clone, Default)]
pub struct PairVersions {
    /// The reference (oracle) engine's release.
    pub reference: Option<EngineVersion>,
    /// The subject (under-test) engine's release.
    pub subject: Option<EngineVersion>,
}

impl PairVersions {
    /// Do the two engines emulate **different** Tcl release lines?
    ///
    /// Compares the release *line*, not the patchlevel: `9.0.4` against a
    /// subject pinned to `9.0` is a matched pair, but `8.6.16` against `9.0.4`
    /// is not. Unknown on either side is *not* skew — an unrecognised
    /// patchlevel is reported, never guessed at.
    #[must_use]
    pub fn skewed(&self) -> bool {
        match (
            self.reference.as_ref().and_then(|v| v.line),
            self.subject.as_ref().and_then(|v| v.line),
        ) {
            (Some(r), Some(s)) => r != s,
            _ => false,
        }
    }

    /// A one-line human warning when the pair is version-skewed, else `None`.
    #[must_use]
    pub fn skew_warning(&self) -> Option<String> {
        if !self.skewed() {
            return None;
        }
        let r = self.reference.as_ref()?;
        let s = self.subject.as_ref()?;
        Some(format!(
            "WARNING: version-skewed pair — reference is Tcl {} but subject emulates Tcl {}.\n\
             Deliberate cross-version semantic changes will be recorded as findings.\n\
             Every finding from issue #1328's campaign was this mistake. Either point\n\
             --tclsh at a matching release, or pin the subject with --subject-tcl-version.",
            r.patchlevel, s.patchlevel,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_maps_a_patchlevel_to_its_release_line() {
        assert_eq!(
            EngineVersion::parse("9.0.4"),
            EngineVersion {
                patchlevel: "9.0.4".to_owned(),
                line: Some(TclVersion::V9_0)
            }
        );
        assert_eq!(
            EngineVersion::parse("8.6.16").line,
            Some(TclVersion::V8_6),
            "8.6.16 is the 8.6 line"
        );
    }

    #[test]
    fn parse_trims_and_survives_an_unmodelled_release() {
        assert_eq!(EngineVersion::parse("  9.0.4\n").patchlevel, "9.0.4");
        // 8.0 is outside the modelled 8.4–9.1 range: recorded, not guessed.
        let old = EngineVersion::parse("8.0.5");
        assert_eq!(old.patchlevel, "8.0.5");
        assert_eq!(old.line, None);
    }

    /// The exact shape of issue #1328: an 8.6 oracle against a 9.0 subject.
    #[test]
    fn an_86_reference_against_a_90_subject_is_skew() {
        let p = PairVersions {
            reference: Some(EngineVersion::parse("8.6.16")),
            subject: Some(EngineVersion::parse("9.0.4")),
        };
        assert!(p.skewed());
        let w = p.skew_warning().expect("skew warns");
        assert!(w.contains("8.6.16") && w.contains("9.0.4"), "{w}");
    }

    #[test]
    fn a_matched_pair_is_not_skew_and_does_not_warn() {
        let p = PairVersions {
            reference: Some(EngineVersion::parse("9.0.4")),
            // A different patch level of the same line is still a match.
            subject: Some(EngineVersion::parse("9.0.1")),
        };
        assert!(!p.skewed());
        assert!(p.skew_warning().is_none());
    }

    #[test]
    fn an_unknown_version_is_not_treated_as_skew() {
        let p = PairVersions {
            reference: Some(EngineVersion::parse("weird")),
            subject: Some(EngineVersion::parse("9.0.4")),
        };
        assert!(!p.skewed(), "an unparsed version must not claim skew");
        assert!(p.skew_warning().is_none());
        assert!(
            !PairVersions::default().skewed(),
            "no probe data is not skew"
        );
    }
}
