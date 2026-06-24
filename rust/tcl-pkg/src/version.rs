//! Semantic version type with Tcl-friendly parsing.
//!
//! Version ordering agrees with
//! semver 2.0 and with C Tcl's `package require` ordering: missing patch digits
//! default to zero, a leading `v` is tolerated, and Tcl-style `a1`/`b2`/`rc1`
//! prereleases collate alongside semver's `-alpha.1` form (`8.6.3a1` sorts
//! before `8.6.3`).

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
        let major = caps["major"].parse::<u64>().expect("regex digits parse");
        let minor = caps
            .name("minor")
            .map_or(0, |m| m.as_str().parse().expect("regex digits parse"));
        let patch = caps
            .name("patch")
            .map_or(0, |m| m.as_str().parse().expect("regex digits parse"));
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
