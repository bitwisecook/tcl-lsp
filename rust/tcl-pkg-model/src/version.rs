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

//! tclpkg release tags and Tcl dependency requirements.
//!
//! Release tags and legacy minimums use `SemVer` with optional patch digits,
//! a leading `v`, and short `a1`/`b2`/`rc1` prereleases. Tcl dependency
//! operands use the interpreter's version grammar from `tcl-dialect`.

use std::cmp::Ordering;
use std::fmt;
use std::sync::LazyLock;

use regex::Regex;

/// Raised when a version string cannot be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionError(pub String);

impl fmt::Display for VersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for VersionError {}

// Core semver + lax trailing-zero + optional v-prefix + Tcl-style alpha/beta.
static SEMVER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)^
        v?
        (?P<major>0|[1-9]\d*)
        (?:\.(?P<minor>0|[1-9]\d*))?
        (?:\.(?P<patch>0|[1-9]\d*))?
        (?:
            (?P<prerelease>
                -[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*
              | [ab]\d+
              | rc\d+
            )
        )?
        (?:\+(?P<build>[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?
        $",
    )
    .expect("version regex compiles")
});

static TCL_PRE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(a|b|rc)(\d+)$").expect("tcl prerelease regex compiles"));

/// One comparable piece of a prerelease tag.
///
/// Numeric pieces sort before alphabetic ones (semver rule); the [`Sentinel`]
/// stands in for "no prerelease" so a release sorts after any prerelease.
///
/// [`Sentinel`]: PreItem::Sentinel
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PreItem {
    Num(u64),
    Alpha(String),
    Sentinel,
}

impl PreItem {
    const fn rank(&self) -> u8 {
        match self {
            // Tag scheme: numeric items sort before alpha, alpha before the sentinel.
            PreItem::Num(_) => 0,
            PreItem::Alpha(_) => 1,
            PreItem::Sentinel => 2,
        }
    }
}

impl Ord for PreItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank()
            .cmp(&other.rank())
            .then_with(|| match (self, other) {
                (PreItem::Num(a), PreItem::Num(b)) => a.cmp(b),
                (PreItem::Alpha(a), PreItem::Alpha(b)) => a.cmp(b),
                _ => Ordering::Equal,
            })
    }
}

impl PartialOrd for PreItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A semver-compatible version with Tcl-friendly parsing.
///
/// Instances are immutable, hashable, and totally ordered. Comparison follows
/// semver 2.0 with Tcl-style short prereleases mapped onto their canonical
/// semver form before comparison; build metadata never affects ordering.
#[derive(Debug, Clone)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    /// Comparable prerelease pieces (empty when there is no prerelease).
    prerelease: Vec<PreItem>,
    /// Canonical prerelease string, e.g. `-alpha.1` (empty when none).
    pub prerelease_raw: String,
    pub build: String,
    pub original: String,
}

impl Version {
    /// Parse a version string, applying the Tcl-friendly lax rules.
    pub fn parse(raw: &str) -> Result<Version, VersionError> {
        let stripped = raw.trim();
        if stripped.is_empty() {
            return Err(VersionError("empty version string".to_string()));
        }
        let caps = SEMVER_RE
            .captures(stripped)
            .ok_or_else(|| VersionError(format!("invalid version: '{raw}'")))?;
        let component = |name| {
            caps.name(name)
                .map_or(Ok(0), |matched| matched.as_str().parse::<u64>())
                .map_err(|_| VersionError(format!("version component out of range: '{raw}'")))
        };
        let major = component("major")?;
        let minor = component("minor")?;
        let patch = component("patch")?;
        let (prerelease, prerelease_raw) = match caps.name("prerelease") {
            Some(m) => parse_prerelease(m.as_str()),
            None => (Vec::new(), String::new()),
        };
        let build = caps
            .name("build")
            .map_or(String::new(), |m| m.as_str().to_string());
        Ok(Version {
            major,
            minor,
            patch,
            prerelease,
            prerelease_raw,
            build,
            original: stripped.to_string(),
        })
    }

    /// Return the canonical `MAJOR.MINOR.PATCH[-pre][+build]` form.
    #[must_use]
    pub fn canonical(&self) -> String {
        let mut base = format!("{}.{}.{}", self.major, self.minor, self.patch);
        if !self.prerelease_raw.is_empty() {
            base.push_str(&self.prerelease_raw);
        }
        if !self.build.is_empty() {
            base.push('+');
            base.push_str(&self.build);
        }
        base
    }

    /// The comparison key: prerelease versions sort before the release, with a
    /// sentinel standing in for "no prerelease".
    fn compare_key(&self) -> (u64, u64, u64, Vec<PreItem>) {
        let pre = if self.prerelease.is_empty() {
            vec![PreItem::Sentinel]
        } else {
            self.prerelease.clone()
        };
        (self.major, self.minor, self.patch, pre)
    }
}

fn parse_prerelease(raw: &str) -> (Vec<PreItem>, String) {
    if let Some(rest) = raw.strip_prefix('-') {
        let canonical = format!("-{rest}");
        let comparable = rest
            .split('.')
            .map(|piece| {
                if !piece.is_empty() && piece.bytes().all(|b| b.is_ascii_digit()) {
                    PreItem::Num(piece.parse().unwrap_or(0))
                } else {
                    PreItem::Alpha(piece.to_string())
                }
            })
            .collect();
        return (comparable, canonical);
    }

    if let Some(caps) = TCL_PRE_RE.captures(raw) {
        let (label, order) = match &caps[1] {
            "a" => ("alpha", 0u64),
            "b" => ("beta", 1u64),
            _ => ("rc", 2u64),
        };
        let number: u64 = caps[2].parse().unwrap_or(0);
        let comparable = vec![PreItem::Num(order), PreItem::Num(number)];
        return (comparable, format!("-{label}.{number}"));
    }

    (vec![PreItem::Alpha(raw.to_string())], raw.to_string())
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.canonical())
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.compare_key() == other.compare_key()
    }
}

impl Eq for Version {}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.compare_key().cmp(&other.compare_key())
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl std::hash::Hash for Version {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.compare_key().hash(state);
    }
}

/// The Tcl grammar admitted by a package manifest and its portable lock data.
const MANIFEST_TCL: tcl_dialect::TclVersion = tcl_dialect::TclVersion::V9_1;

/// One package-manager dependency. Authored package minimums retain the
/// tclpkg version convention. Imported Tcl requirements retain the distinct
/// `package require` matching rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyVersion {
    /// A legacy tclpkg minimum version.
    Minimum(Version),
    /// Alternatives or exact equality in the Tcl package grammar.
    Tcl(TclPackageRequirement),
}

/// A validated Tcl dependency as a single manifest operand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TclPackageRequirement {
    versions: Vec<String>,
    exact: bool,
    release: tcl_dialect::TclVersion,
}

impl TclPackageRequirement {
    /// Retain each package requirement without converting its endpoints.
    ///
    /// # Errors
    /// An empty alternative list, malformed requirement, or exact request
    /// without one valid version is refused.
    pub fn new(
        versions: Vec<String>,
        exact: bool,
        release: Option<tcl_dialect::TclVersion>,
    ) -> Result<Self, VersionError> {
        let release = release.unwrap_or(MANIFEST_TCL);
        let valid = if exact {
            matches!(versions.as_slice(), [version] if tcl_dialect::validate_version_for(version, release))
        } else {
            !versions.is_empty()
                && (release.has_package_requirements() || versions.len() == 1)
                && versions
                    .iter()
                    .all(|version| tcl_dialect::validate_requirement_for(version, release).is_ok())
        };
        if !valid {
            return Err(VersionError(format!(
                "invalid Tcl package requirement: {versions:?}"
            )));
        }
        Ok(Self {
            versions,
            exact,
            release,
        })
    }

    /// Candidate versions named by this dependency.
    ///
    /// The tclpkg source registry describes locations. It has no version
    /// catalogue, so the resolver can consider only concrete versions named
    /// by the graph or its root overrides.
    pub fn candidates(&self) -> impl Iterator<Item = &str> {
        self.versions.iter().filter_map(|version| {
            if self.exact {
                Some(version.as_str())
            } else {
                tcl_dialect::package_requirement_bounds_for(version, self.release)
                    .map(|(lower, _)| lower)
            }
        })
    }

    /// Whether a concrete candidate satisfies this Tcl dependency.
    #[must_use]
    pub fn matches(&self, candidate: &str) -> bool {
        if self.exact {
            tcl_dialect::version_matches_exact_for(candidate, &self.versions[0], self.release)
        } else {
            self.versions.iter().any(|requirement| {
                tcl_dialect::version_satisfies_for(candidate, requirement, self.release)
            })
        }
    }
}

impl DependencyVersion {
    /// Read one dependency operand from a manifest or lockfile.
    ///
    /// # Errors
    /// The operand must be a tclpkg minimum or a valid Tcl requirement list.
    pub fn parse(raw: &str) -> Result<Self, VersionError> {
        // grammar-bypass-ok: scripts — Selects the operand form before the Tcl list parser.
        if raw.trim_start().starts_with('-') {
            let words = tcl_syntax::list::split_list(raw).map_err(|error| {
                VersionError(format!("invalid requirement list: {}", error.message()))
            })?;
            let Some((mode, versions)) = words.split_first() else {
                return Err(VersionError("missing Tcl package requirement".to_owned()));
            };
            let exact = match mode.as_ref() {
                "-tcl" => false,
                "-exact" => true,
                other => return Err(VersionError(format!("invalid requirement mode: '{other}'"))),
            };
            let (release, versions) = match versions {
                [option, value, rest @ ..] if option == "-release" => {
                    let release = tcl_dialect::TclVersion::ALL
                        .into_iter()
                        .find(|release| release.version_string() == value.as_ref())
                        .ok_or_else(|| {
                            VersionError(format!("unknown Tcl package release: '{value}'"))
                        })?;
                    (Some(release), rest)
                }
                _ => (None, versions),
            };
            return TclPackageRequirement::new(
                versions.iter().map(ToString::to_string).collect(),
                exact,
                release,
            )
            .map(Self::Tcl);
        }
        Version::parse(raw).map(Self::Minimum)
    }
}

impl fmt::Display for DependencyVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Minimum(version) => version.fmt(f),
            Self::Tcl(requirement) => {
                let mode = if requirement.exact { "-exact" } else { "-tcl" };
                let release = (requirement.release != MANIFEST_TCL)
                    .then(|| ["-release", requirement.release.version_string()]);
                f.write_str(&tcl_syntax::list::join_list(
                    std::iter::once(mode)
                        .chain(release.into_iter().flatten())
                        .chain(requirement.versions.iter().map(String::as_str)),
                ))
            }
        }
    }
}

/// Convenience alias for [`Version::parse`].
pub fn parse_version(raw: &str) -> Result<Version, VersionError> {
    Version::parse(raw)
}

/// Return the maximum of a non-empty slice of versions.
pub fn max_version(versions: &[Version]) -> Result<Version, VersionError> {
    versions
        .iter()
        .max()
        .cloned()
        .ok_or_else(|| VersionError("max_version requires a non-empty list".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependency_operands_keep_the_selected_tcl_grammar() {
        let operand = "-tcl -release 8.4 1.2";
        let DependencyVersion::Tcl(requirement) = DependencyVersion::parse(operand).unwrap() else {
            panic!("expected a Tcl requirement");
        };
        assert!(requirement.matches("1.2.0"));
        assert!(!requirement.matches("1.2a1"));
        assert_eq!(DependencyVersion::Tcl(requirement).to_string(), operand);
        assert!(DependencyVersion::parse("-tcl -release 8.4 1.2 2.0").is_err());
        assert!(DependencyVersion::parse("-exact -release 8.6 1.2+data").is_err());
        assert!(DependencyVersion::parse("-exact -release 9.0 1.2+data").is_ok());
    }

    fn v(s: &str) -> Version {
        Version::parse(s).expect("parses")
    }

    #[test]
    fn basic_three_part() {
        let ver = v("1.2.3");
        assert_eq!((ver.major, ver.minor, ver.patch), (1, 2, 3));
        assert_eq!(ver.prerelease_raw, "");
        assert_eq!(ver.build, "");
    }

    #[test]
    fn short_forms_default() {
        assert_eq!((v("1.2").major, v("1.2").minor, v("1.2").patch), (1, 2, 0));
        assert_eq!((v("7").major, v("7").minor, v("7").patch), (7, 0, 0));
    }

    #[test]
    fn v_prefix_allowed() {
        assert_eq!(v("v1.2.3"), v("1.2.3"));
    }

    #[test]
    fn tcl_style_prereleases() {
        assert_eq!(v("8.6.3a1").prerelease_raw, "-alpha.1");
        assert_eq!(v("1.0b2").prerelease_raw, "-beta.2");
        assert_eq!(v("2.0rc1").prerelease_raw, "-rc.1");
        assert_eq!(v("1.2.3-rc.4").prerelease_raw, "-rc.4");
    }

    #[test]
    fn build_metadata() {
        assert_eq!(v("1.2.3+build.42").build, "build.42");
    }

    #[test]
    fn parse_errors() {
        assert!(Version::parse("").is_err());
        assert!(Version::parse("not a version").is_err());
    }

    #[test]
    fn canonical_forms() {
        assert_eq!(v("1.2.3").canonical(), "1.2.3");
        assert_eq!(v("1.2").canonical(), "1.2.0");
        assert_eq!(v("v1.2.3").canonical(), "1.2.3");
        assert_eq!(v("8.6.3a1").canonical(), "8.6.3-alpha.1");
        assert_eq!(v("1.2.3+foo").canonical(), "1.2.3+foo");
        assert_eq!(v("1.2").to_string(), "1.2.0");
    }

    #[test]
    fn ordering() {
        assert!(v("2.0.0") > v("1.99.99"));
        assert!(v("1.3.0") > v("1.2.99"));
        assert!(v("1.2.3") > v("1.2.2"));
        assert!(v("1.2.3-rc.1") < v("1.2.3"));
        assert!(v("8.6.3a1") < v("8.6.3b1"));
        assert!(v("8.6.3b2") < v("8.6.3rc1"));
        assert!(v("8.6.3rc1") < v("8.6.3"));
        assert!(v("1.0.0-rc.2") > v("1.0.0-rc.1"));
        assert_eq!(v("1.2.3+a"), v("1.2.3+b"));
    }

    #[test]
    fn sorting_mixed_forms() {
        let mut versions = [v("1.0.0"), v("1.2"), v("1.2.3a1"), v("v1.2.3"), v("0.9.9")];
        versions.sort();
        let strs: Vec<String> = versions.iter().map(ToString::to_string).collect();
        assert_eq!(
            strs,
            vec!["0.9.9", "1.0.0", "1.2.0", "1.2.3-alpha.1", "1.2.3"]
        );
    }

    #[test]
    fn hashing_and_dedup() {
        use std::collections::HashSet;
        let set: HashSet<Version> = [v("1.0.0"), v("v1.0.0"), v("1.0")].into_iter().collect();
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn max_version_helper() {
        let chosen = max_version(&[v("1.0.0"), v("1.2.3"), v("1.1.9")]).unwrap();
        assert_eq!(chosen.to_string(), "1.2.3");
        assert!(max_version(&[]).is_err());
    }
}
