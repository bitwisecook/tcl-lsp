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

//! Tcl release metadata for differential backends.
//!
//! A backend mismatch is not a defect when the two engines intentionally
//! implement different Tcl release lines. Probe each engine through the same
//! subprocess harness as the campaign, warn before a skewed run, and persist
//! the raw patchlevels on every finding so later triage retains that context.

use std::path::Path;
use std::time::Duration;

use tcl_dialect::TclVersion;

/// One engine's self-reported Tcl release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineVersion {
    /// Raw `[info patchlevel]` output.
    pub patchlevel: String,
    /// Modelled major/minor release line.
    pub line: Option<TclVersion>,
}

impl EngineVersion {
    /// Parse one patchlevel, preserving an unmodelled value verbatim.
    #[must_use]
    pub fn parse(patchlevel: &str) -> Self {
        let patchlevel = patchlevel.trim().to_owned();
        let line = TclVersion::from_package_version(&patchlevel);
        Self { patchlevel, line }
    }

    /// Probe `[info patchlevel]` through a resolved backend invocation.
    #[must_use]
    pub fn probe(binary: &Path, args: &[String], timeout: Duration) -> Option<Self> {
        let dir = std::env::temp_dir().join(format!(
            "tcl-fuzz-version-{}-{}",
            std::process::id(),
            binary
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("backend")
        ));
        std::fs::create_dir_all(&dir).ok()?;
        let script = dir.join("probe.tcl");
        std::fs::write(&script, "puts [info patchlevel]\n").ok()?;
        let outcome = crate::harness::run_backend(binary, args, &script, timeout);
        let _ = std::fs::remove_file(script);
        let _ = std::fs::remove_dir(dir);
        let crate::harness::Outcome::Ran {
            stdout,
            errored: false,
            ..
        } = outcome
        else {
            return None;
        };
        let patchlevel = stdout.lines().next()?.trim();
        (!patchlevel.is_empty()).then(|| Self::parse(patchlevel))
    }
}

/// Release metadata for a differential pair.
#[derive(Debug, Clone, Default)]
pub struct PairVersions {
    /// Presumed-correct reference release.
    pub reference: Option<EngineVersion>,
    /// Subject release under test.
    pub subject: Option<EngineVersion>,
}

impl PairVersions {
    /// Whether both engines reported different patchlevels.
    ///
    /// Even engines on the same major/minor line can differ at a bug-fix
    /// boundary, so retain and compare the full value rather than treating
    /// `9.0.1` and `9.0.4` as an automatically fair pair.
    #[must_use]
    pub fn skewed(&self) -> bool {
        match (&self.reference, &self.subject) {
            (Some(reference), Some(subject)) => reference.patchlevel != subject.patchlevel,
            _ => false,
        }
    }

    /// Human-readable skew warning, when both reported patchlevels differ.
    #[must_use]
    pub fn skew_warning(&self) -> Option<String> {
        if !self.skewed() {
            return None;
        }
        Some(format!(
            "WARNING: version-skewed pair — reference Tcl {}, subject Tcl {}; deliberate cross-version semantics can appear as findings",
            self.reference.as_ref()?.patchlevel,
            self.subject.as_ref()?.patchlevel,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patchlevels_map_to_release_lines() {
        assert_eq!(EngineVersion::parse("9.0.4").line, Some(TclVersion::V9_0));
        assert_eq!(EngineVersion::parse("8.6.16").line, Some(TclVersion::V8_6));
        assert_eq!(EngineVersion::parse("unmodelled").line, None);
    }

    #[test]
    fn skew_needs_two_different_reported_patchlevels() {
        let skewed = PairVersions {
            reference: Some(EngineVersion::parse("8.6.16")),
            subject: Some(EngineVersion::parse("9.0.4")),
        };
        assert!(skewed.skewed());
        assert!(skewed.skew_warning().is_some());

        let same_line_different_patch = PairVersions {
            reference: Some(EngineVersion::parse("9.0.1")),
            subject: Some(EngineVersion::parse("9.0.4")),
        };
        assert!(same_line_different_patch.skewed());

        let identical = PairVersions {
            reference: Some(EngineVersion::parse("9.0.4")),
            subject: Some(EngineVersion::parse("9.0.4")),
        };
        assert!(!identical.skewed());
        assert!(!PairVersions::default().skewed());
    }
}
