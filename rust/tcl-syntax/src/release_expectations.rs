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

//! Per-release expectation columns for the conformance vector files.
//!
//! Every vector file in `tests/data/` ends each row with an expectation
//! field.  A behaviour that all five modelled releases agree on is written
//! as a single value; a behaviour that *differs* per release is written as
//! a semicolon-separated list of `RANGE=VALUE` entries:
//!
//! ```text
//! 8.4-8.6={0 20};9.0+={1 {can't read "v": no such variable}}
//! ```
//!
//! Ranges are inclusive over the release ladder [`LADDER`]
//! (8.4, 8.5, 8.6, 9.0, 9.1) and are written as a single release (`8.4`),
//! a closed range (`8.4-8.6`), or an open-ended range (`9.0+`).  The
//! entries must together cover every release on the ladder exactly once —
//! a gap or an overlap is a bug in the row, not an input condition, so
//! [`PerRelease::parse`] reports it.
//!
//! Splitting is deliberately conservative: a `;` only separates entries
//! when the text after it is a range token followed by `=`, so expectation
//! values may contain semicolons.

use tcl_dialect::TclVersion;

/// The release ladder every expectation column is keyed by, oldest first.
pub const LADDER: [TclVersion; 5] = TclVersion::ALL;

/// The newest release on [`LADDER`] — the column the pure (release-agnostic)
/// consumers assert against.
#[must_use]
pub fn newest_release() -> TclVersion {
    LADDER[LADDER.len() - 1]
}

/// Position of `release` on [`LADDER`].
fn ladder_index(release: TclVersion) -> usize {
    LADDER
        .iter()
        .position(|candidate| *candidate == release)
        .expect("every TclVersion is on the ladder")
}

/// One expectation field: the expected observable for each release on
/// [`LADDER`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerRelease {
    values: [String; LADDER.len()],
    tagged: bool,
}

impl PerRelease {
    /// Parse one expectation field.
    ///
    /// # Errors
    /// Returns a human-readable message when the field is a malformed
    /// `RANGE=VALUE` list — an unknown release, an inverted range, an
    /// overlap, or a release the entries do not cover.
    pub fn parse(field: &str) -> Result<Self, String> {
        let field = field.trim();
        if range_prefix_len(field).is_none() {
            return Ok(Self {
                values: core::array::from_fn(|_| field.to_owned()),
                tagged: false,
            });
        }
        let mut values: [Option<String>; LADDER.len()] = core::array::from_fn(|_| None);
        for entry in split_entries(field) {
            let (range, value) = entry
                .split_once('=')
                .ok_or_else(|| format!("expectation entry {entry:?} has no `=`"))?;
            let (lo, hi) = parse_range(range.trim())?;
            for slot in lo..=hi {
                if values[slot].is_some() {
                    return Err(format!(
                        "release {} is covered twice in {field:?}",
                        LADDER[slot].version_string()
                    ));
                }
                values[slot] = Some(value.trim().to_owned());
            }
        }
        let mut out: [String; LADDER.len()] = core::array::from_fn(|_| String::new());
        for (slot, value) in values.into_iter().enumerate() {
            out[slot] = value.ok_or_else(|| {
                format!(
                    "release {} has no expectation in {field:?}",
                    LADDER[slot].version_string()
                )
            })?;
        }
        Ok(Self {
            values: out,
            tagged: true,
        })
    }

    /// The expected observable for `release`.
    #[must_use]
    pub fn get(&self, release: TclVersion) -> &str {
        &self.values[ladder_index(release)]
    }

    /// The expected observable for the newest modelled release.
    #[must_use]
    pub fn newest(&self) -> &str {
        &self.values[LADDER.len() - 1]
    }

    /// Whether the row was written with release-tagged columns (as opposed
    /// to one value every release shares).
    #[must_use]
    pub fn is_release_tagged(&self) -> bool {
        self.tagged
    }

    /// Whether every release expects the same observable.
    #[must_use]
    pub fn is_uniform(&self) -> bool {
        self.values.iter().all(|value| value == &self.values[0])
    }
}

/// Length of a leading range token (`8.4`, `8.4-8.6`, `9.0+`) when the field
/// is in `RANGE=VALUE` form, or `None` when it is a plain single value.
fn range_prefix_len(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut end = 0;
    while end < bytes.len() && matches!(bytes[end], b'0'..=b'9' | b'.' | b'+' | b'-') {
        end += 1;
    }
    (end > 0 && bytes.get(end) == Some(&b'=')).then_some(end)
}

/// Split a `RANGE=VALUE;RANGE=VALUE` field at the `;` that precede a range
/// token, leaving semicolons inside expectation values alone.
fn split_entries(field: &str) -> Vec<&str> {
    let mut entries = Vec::new();
    let mut start = 0;
    for (offset, _) in field.match_indices(';') {
        if range_prefix_len(field[offset + 1..].trim_start()).is_some() {
            entries.push(field[start..offset].trim());
            start = offset + 1;
        }
    }
    entries.push(field[start..].trim());
    entries
}

/// Parse an inclusive range token into ladder slot bounds.
fn parse_range(range: &str) -> Result<(usize, usize), String> {
    if let Some(low) = range.strip_suffix('+') {
        return Ok((release_slot(low)?, LADDER.len() - 1));
    }
    let Some((low, high)) = range.split_once('-') else {
        let slot = release_slot(range)?;
        return Ok((slot, slot));
    };
    let (low, high) = (release_slot(low)?, release_slot(high)?);
    if low > high {
        return Err(format!("range {range:?} runs backwards"));
    }
    Ok((low, high))
}

/// Ladder slot for a `major.minor` release name.
fn release_slot(name: &str) -> Result<usize, String> {
    LADDER
        .iter()
        .position(|release| release.version_string() == name.trim())
        .ok_or_else(|| format!("unknown release {name:?} (ladder: 8.4, 8.5, 8.6, 9.0, 9.1)"))
}

#[cfg(test)]
mod tests {
    use super::{LADDER, PerRelease};
    use tcl_dialect::TclVersion;

    #[test]
    fn a_single_value_applies_to_every_release() {
        let expectation = PerRelease::parse("::helper").expect("parses");
        assert!(!expectation.is_release_tagged());
        assert!(expectation.is_uniform());
        for release in LADDER {
            assert_eq!(expectation.get(release), "::helper");
        }
    }

    #[test]
    fn tagged_columns_cover_the_ladder() {
        let expectation =
            PerRelease::parse("8.4-8.6={0 20};9.0+={1 {can't read \"v\": no such variable}}")
                .expect("parses");
        assert!(expectation.is_release_tagged());
        assert!(!expectation.is_uniform());
        assert_eq!(expectation.get(TclVersion::V8_4), "{0 20}");
        assert_eq!(expectation.get(TclVersion::V8_6), "{0 20}");
        assert_eq!(
            expectation.get(TclVersion::V9_0),
            "{1 {can't read \"v\": no such variable}}"
        );
        assert_eq!(expectation.newest(), expectation.get(TclVersion::V9_1));
    }

    #[test]
    fn a_semicolon_inside_a_value_is_not_a_separator() {
        let expectation = PerRelease::parse("8.4=1 {a; b};8.5+=0 {}").expect("parses");
        assert_eq!(expectation.get(TclVersion::V8_4), "1 {a; b}");
        assert_eq!(expectation.get(TclVersion::V8_5), "0 {}");
    }

    #[test]
    fn every_release_must_be_covered_exactly_once() {
        assert!(PerRelease::parse("8.4=x;8.6+=y").is_err());
        assert!(PerRelease::parse("8.4-9.1=x;9.0=y").is_err());
        assert!(PerRelease::parse("8.7+=x").is_err());
        assert!(PerRelease::parse("9.0-8.4=x").is_err());
    }

    #[test]
    fn a_single_release_column_is_written_bare() {
        let expectation = PerRelease::parse("8.4=old;8.5=mid;8.6+=new").expect("parses");
        assert_eq!(expectation.get(TclVersion::V8_4), "old");
        assert_eq!(expectation.get(TclVersion::V8_5), "mid");
        assert_eq!(expectation.get(TclVersion::V8_6), "new");
        assert_eq!(expectation.get(TclVersion::V9_1), "new");
    }
}
