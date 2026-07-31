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

//! The ordered Tcl release enum behaviour semantics key off, and the
//! three-valued policy type for behaviours a non-Tcl profile has no
//! opinion on.

/// A specific Tcl release whose **compile-time** semantics a constant fold may
/// depend on — e.g. `string is integer` is unbounded on 9.0 but caps at
/// `2³²-1` on 8.x, and `string is wideinteger` / `entier` / `dict` and
/// `format %b` don't exist (they *raise*) before a given release.  Ordered, so
/// a fold can test `version >= TclVersion::V8_5`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TclVersion {
    /// Tcl 8.4.
    V8_4,
    /// Tcl 8.5.
    V8_5,
    /// Tcl 8.6.
    V8_6,
    /// Tcl 9.0.
    V9_0,
    /// Tcl 9.1. Shares 9.0's compile-time fold semantics (a superset release);
    /// ordered after `V9_0` so `>= V9_0` gates include it.
    V9_1,
}

impl TclVersion {
    /// Map an optimiser dialect string (`"tcl8.4"` … `"tcl9.1"`) to a version,
    /// or `None` for an unversioned (`"tcl"`), non-Tcl (`"f5-irules"`), or
    /// unknown dialect — in which case a versioned fold must return only the
    /// dialect-invariant subset every release shares.
    #[must_use]
    pub fn from_dialect(dialect: Option<&str>) -> Option<Self> {
        match dialect {
            Some("tcl8.4") => Some(Self::V8_4),
            Some("tcl8.5") => Some(Self::V8_5),
            Some("tcl8.6") => Some(Self::V8_6),
            Some("tcl9.0") => Some(Self::V9_0),
            // 9.1 must not fall through to `None` (which degrades a versioned
            // fold to the dialect-invariant subset) — it behaves as 9.0+
            // (RUST_ISSUE_083).
            Some("tcl9.1") => Some(Self::V9_1),
            _ => None,
        }
    }

    /// Map a `package require Tcl` version string (`"8.6"`, `"9.0"`,
    /// `"8.6.10"`) to the release enum, or `None` for anything the enum
    /// does not model — callers treat `None` as "no floor", never as an
    /// error.
    #[must_use]
    pub fn from_package_version(version: &str) -> Option<Self> {
        let mut parts = version.split('.');
        let major = parts.next()?.parse::<u32>().ok()?;
        let minor = parts.next().and_then(|m| m.parse::<u32>().ok())?;
        match (major, minor) {
            (8, 4) => Some(Self::V8_4),
            (8, 5) => Some(Self::V8_5),
            (8, 6) => Some(Self::V8_6),
            (9, 0) => Some(Self::V9_0),
            (9, 1) => Some(Self::V9_1),
            _ => None,
        }
    }

    /// The `major.minor` string this release reports as
    /// `[package provide Tcl]`.
    ///
    /// The real interpreter reports a full patchlevel (`9.0.4`), but every
    /// requirement form compares major-then-minor first, so the two-component
    /// form answers identically for any requirement that does not name a patch
    /// level — and the enum models no patch levels to name.
    #[must_use]
    pub fn version_string(self) -> &'static str {
        match self {
            Self::V8_4 => "8.4",
            Self::V8_5 => "8.5",
            Self::V8_6 => "8.6",
            Self::V9_0 => "9.0",
            Self::V9_1 => "9.1",
        }
    }

    /// Does this release satisfy *any* of `requirements` — the answer
    /// `package vsatisfies [package provide Tcl] REQ ?REQ …?` gives?
    ///
    /// An empty `requirements` list is `false`: real `package vsatisfies`
    /// rejects it as a wrong-argument-count error, and no caller here has a
    /// meaningful "satisfies nothing" question to ask.
    #[must_use]
    pub fn satisfies_any<S: AsRef<str>>(self, requirements: &[S]) -> bool {
        requirements
            .iter()
            .any(|r| version_satisfies(self.version_string(), r.as_ref()))
    }

    /// [`Self::satisfies_any`], but [`Ternary::Inert`] when the answer depends
    /// on a **patch level** this enum does not model.
    ///
    /// `version_string` is `major.minor`, so a requirement naming a third
    /// component is compared against `9.0` rather than the real `9.0.4`:
    /// `vsatisfies 9.0.1-` reads as *unsatisfied* even though every shipped
    /// 9.0 release from 9.0.1 on satisfies it.  That is the one direction that
    /// turns into a false "this package cannot load" downstream, so a caller
    /// that can abstain must, rather than take the wrong answer
    /// ([`requirement_names_patch_level`]).
    ///
    /// The OR short-circuits exactly as `package vsatisfies` does: a
    /// two-component requirement that *is* satisfied settles the whole test
    /// [`Ternary::Yes`] however many patch-level requirements sit beside it.
    /// Abstention is deliberately coarse — `9.0.1-` is undecidable even for a
    /// target the `major.minor` comparison alone would settle (8.6 cannot
    /// satisfy it) — because "conditional" is the safe direction and the
    /// precision is not worth a second comparison rule.
    #[must_use]
    pub fn satisfies_any_ternary<S: AsRef<str>>(self, requirements: &[S]) -> Ternary {
        let mut undecidable = false;
        for requirement in requirements {
            let requirement = requirement.as_ref();
            if requirement_names_patch_level(requirement) {
                undecidable = true;
            } else if version_satisfies(self.version_string(), requirement) {
                return Ternary::Yes;
            }
        }
        if undecidable {
            Ternary::Inert
        } else {
            Ternary::No
        }
    }
}

/// Whether a `package vsatisfies` requirement names a patch level — three or
/// more dotted components in either bound (`9.0.1`, `9.0.1-`, `8.6-9.0.2`).
///
/// [`TclVersion`] models `major.minor` releases only, so such a requirement
/// cannot be evaluated against it faithfully; see
/// [`TclVersion::satisfies_any_ternary`].
#[must_use]
pub fn requirement_names_patch_level(requirement: &str) -> bool {
    let requirement = requirement.trim();
    let (lo, hi) = match requirement.split_once('-') {
        Some((lo, hi)) => (lo.trim(), hi.trim()),
        None => (requirement, ""),
    };
    [lo, hi]
        .into_iter()
        .filter(|bound| !bound.is_empty())
        .any(|bound| bound.split('.').count() >= 3)
}

// ---------------------------------------------------------------------------
// The package version comparator — a port of C Tcl's `generic/tclPkg.c`.
//
// One implementation, shared by the bytecode VM's `package vcompare` /
// `vsatisfies` / `require`, the language server's `pkgIndex.tcl` guard
// evaluation, and the package resolver's provider selection, so none of them
// can disagree about what a version or a requirement means.
//
// The three C functions this mirrors, and where each lands here:
//
// | `tclPkg.c`                | here                       |
// |---------------------------|----------------------------|
// | `CheckVersionAndConvert`  | [`ParsedVersion::parse`]   |
// | `CompareVersions`         | [`compare_internal`]       |
// | `RequirementSatisfied`    | [`satisfies_internal`]     |
// | `SelectPackage` (best/best-stable loop) | [`select_package_version`] |
// ---------------------------------------------------------------------------

/// The segment `CheckVersionAndConvert` substitutes for an `a` (alpha)
/// separator, and the pad `RequirementSatisfied` appends to a bound so an
/// unstable release of the bound's own version still counts as at-or-above it.
const SEG_ALPHA: i64 = -2;
/// The segment substituted for a `b` (beta) separator.
const SEG_BETA: i64 = -1;
/// The segment substituted for a `.` separator.
const SEG_DOT: i64 = 0;

/// A version in C Tcl's *internal representation*: the sequence of numbers
/// `CheckVersionAndConvert` builds, where every separator becomes a number of
/// its own (`.` → `0`, `a` → `-2`, `b` → `-1`).
///
/// `1.2` is `[1, 0, 2]`, `1.2a1` is `[1, 0, 2, -2, 1]`, `1.2.3` is
/// `[1, 0, 2, 0, 3]`. Encoding the separator as a *number* is what makes an
/// alpha/beta release order below the corresponding dotted patch release
/// without a second comparison rule: `1.2a1` < `1.2.1` falls straight out of
/// `-2 < 0`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedVersion {
    /// The internal-rep segments.
    segments: Vec<i64>,
    /// `false` when an `a` or `b` separator occurred — C Tcl's `hasunstable`,
    /// the flag `SelectPackage` uses to maintain its "best stable" candidate.
    stable: bool,
}

impl ParsedVersion {
    /// Validate and convert `string`, or `None` when it is not a well-formed
    /// version — a port of `CheckVersionAndConvert`'s TIP 268 rules:
    ///
    /// 1. the first character must be a digit;
    /// 2. every other character must be a digit, `.`, `a`, or `b`;
    /// 3. only one of `a` / `b` may occur;
    /// 4. neither `a`, `b`, nor `.` may sit next to a `.`;
    /// 5. the last character may not be a separator.
    ///
    /// Real Tcl raises `expected version number but got "…"` where this
    /// answers `None`; every caller here turns that into the conservative
    /// static answer (unsatisfiable / unselectable) rather than a panic.
    fn parse(string: &str) -> Option<Self> {
        let mut chars = string.chars();
        let first = chars.next()?;
        if !first.is_ascii_digit() {
            return None;
        }
        let mut segments = Vec::new();
        let mut current: i64 = i64::from(first as u8 - b'0');
        let mut has_unstable = false;
        let mut prev = first;
        for c in chars {
            if c.is_ascii_digit() {
                current = current
                    .saturating_mul(10)
                    .saturating_add(i64::from(c as u8 - b'0'));
                prev = c;
                continue;
            }
            let separator = match c {
                '.' => SEG_DOT,
                'a' => SEG_ALPHA,
                'b' => SEG_BETA,
                _ => return None,
            };
            // Rule 3 — a second `a`/`b`; rules 4 — a separator adjacent to a
            // `.`, or a `.` adjacent to any separator.
            if (has_unstable && c != '.') || (matches!(prev, 'a' | 'b') && c == '.') || prev == '.'
            {
                return None;
            }
            has_unstable |= c != '.';
            segments.push(current);
            segments.push(separator);
            current = 0;
            prev = c;
        }
        // Rule 5 — a trailing separator.
        if matches!(prev, '.' | 'a' | 'b') {
            return None;
        }
        segments.push(current);
        Some(Self {
            segments,
            stable: !has_unstable,
        })
    }

    /// A best-effort rep for a string [`Self::parse`] rejects, so the *total*
    /// [`compare_versions`] never has to invent an answer out of nothing: digit
    /// runs become segments, recognised separators become their codes, and any
    /// other character is skipped. Never used to decide satisfaction — only to
    /// order two strings that are not versions in the first place.
    fn lenient(string: &str) -> Vec<i64> {
        let mut segments = Vec::new();
        let mut current: Option<i64> = None;
        for c in string.chars() {
            if let Some(d) = c.to_digit(10) {
                let acc = current.unwrap_or(0);
                current = Some(acc.saturating_mul(10).saturating_add(i64::from(d)));
                continue;
            }
            let separator = match c {
                '.' => SEG_DOT,
                'a' => SEG_ALPHA,
                'b' => SEG_BETA,
                _ => continue,
            };
            segments.push(current.take().unwrap_or(0));
            segments.push(separator);
        }
        segments.push(current.unwrap_or(0));
        segments
    }
}

/// Compare two internal reps, returning the ordering plus whether the deciding
/// difference was in the **first** segment (C Tcl's `isMajorPtr` out-param).
///
/// A rep shorter than the other is padded with `0` — the exact effect of the
/// C loop, which runs off the end of the shorter string and then compares an
/// empty number against the other side: empty ties with a literal `0` (the
/// leading-zero skip makes both empty), sorts *below* any positive segment
/// (shorter string is the smaller number), and *above* a negative one (the
/// sign shortcut). `0` reproduces all three.
fn compare_internal(a: &[i64], b: &[i64]) -> (core::cmp::Ordering, bool) {
    use core::cmp::Ordering;
    for i in 0..a.len().max(b.len()) {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            Ordering::Equal => {}
            other => return (other, i == 0),
        }
    }
    (Ordering::Equal, false)
}

/// Compare two version numbers exactly as `package vcompare` does.
///
/// Trailing zero components are *not* significant (`1.2` == `1.2.0`), and an
/// alpha/beta release orders below the release it leads up to but above the
/// previous one (`1.2` < `1.2b1` is false, `1.2` < `1.3b1` is true) — both
/// pinned against `tclsh8.6` and `tclsh9.0` in
/// `tcl-dialect/tests/package_version_oracle.rs`.
///
/// Total: a string that is not a well-formed version is compared through a
/// best-effort rep rather than raising, because callers order provider lists
/// that a recorder has already filtered and have no error channel.
#[must_use]
pub fn compare_versions(a: &str, b: &str) -> core::cmp::Ordering {
    let va = ParsedVersion::parse(a).map_or_else(|| ParsedVersion::lenient(a), |p| p.segments);
    let vb = ParsedVersion::parse(b).map_or_else(|| ParsedVersion::lenient(b), |p| p.segments);
    compare_internal(&va, &vb).0
}

/// Whether `version` is a **stable** release — i.e. carries no `a`/`b`
/// (alpha/beta) separator. An unparseable version is not stable.
///
/// This is C Tcl's `hasunstable` flag, the input to `package prefer`'s choice
/// between the best and the best-stable candidate.
#[must_use]
pub fn version_is_stable(version: &str) -> bool {
    ParsedVersion::parse(version).is_some_and(|p| p.stable)
}

/// Does the concrete `version` satisfy one `package vsatisfies` requirement?
///
/// The requirement forms Tcl accepts (`package(n)`, every row verified against
/// `tclsh8.6` 8.6.14 and `tclsh9.0` 9.0.4 — byte-identical):
///
/// | Written   | Means                                             | `8.6` | `9.0` |
/// |-----------|---------------------------------------------------|-------|-------|
/// | `8.5`     | `[8.5, 9)` — up to but excluding the *next major* | yes   | no    |
/// | `8.5-`    | `[8.5, ∞)`                                        | yes   | yes   |
/// | `8.5-9.0` | `[8.5, 9.0)`                                      | yes   | no    |
/// | `8.5-8.5` | exactly 8.5 (what `package require -exact` builds) | no    | no    |
///
/// Every bound that is not an exact `min-max` pair is padded with an alpha
/// segment before comparing, which is why an unstable release of the bound's
/// own version satisfies it: `package vsatisfies 1.2a1 1.2` is **1**, and so
/// is `1.2a1 1.2-1.3`. The `min == max` (exact) form is the one place that
/// padding is skipped, so `-exact 1.2` accepts `1.2` and `1.2.0` but not
/// `1.2a1`.
///
/// An ill-formed version or requirement — which real Tcl raises on — answers
/// `false`: the conservative static reading, since nothing can be shown to
/// satisfy a requirement that cannot be parsed. Nothing is trimmed first, so
/// `" 1.2"` is rejected exactly as the interpreter rejects it.
#[must_use]
pub fn version_satisfies(version: &str, requirement: &str) -> bool {
    let Some(have) = ParsedVersion::parse(version) else {
        return false;
    };
    satisfies_internal(&have.segments, requirement)
}

/// [`version_satisfies`] against an already-parsed candidate — the form
/// [`select_package_version`] needs so a candidate is converted once for the
/// whole requirement list.
fn satisfies_internal(have: &[i64], requirement: &str) -> bool {
    use core::cmp::Ordering;
    let Some((lo, hi)) = requirement.split_once('-') else {
        // No dash: a simple version. The requirement is padded with an alpha
        // segment, and the candidate must be equal or greater *without* the
        // difference landing in the major component — which is what bounds a
        // bare `X.Y` at the next major without naming an upper bound.
        let Some(mut req) = ParsedVersion::parse(requirement).map(|p| p.segments) else {
            return false;
        };
        req.push(SEG_ALPHA);
        let (ord, is_major) = compare_internal(have, &req);
        return ord == Ordering::Equal || (ord == Ordering::Greater && !is_major);
    };
    // `CheckRequirement`: at most one dash.
    if hi.contains('-') {
        return false;
    }
    let Some(min) = ParsedVersion::parse(lo).map(|p| p.segments) else {
        return false;
    };
    if hi.is_empty() {
        // `min-` — open-ended above.
        let mut min = min;
        min.push(SEG_ALPHA);
        return compare_internal(have, &min).0 != Ordering::Less;
    }
    let Some(max) = ParsedVersion::parse(hi).map(|p| p.segments) else {
        return false;
    };
    if compare_internal(&min, &max).0 == Ordering::Equal {
        // `v-v` — the exact form. Compared unpadded, so an alpha/beta release
        // of `v` does *not* satisfy it.
        return compare_internal(&min, have).0 == Ordering::Equal;
    }
    let (mut min, mut max) = (min, max);
    min.push(SEG_ALPHA);
    max.push(SEG_ALPHA);
    compare_internal(&min, have).0 != Ordering::Greater
        && compare_internal(have, &max).0 == Ordering::Less
}

/// The requirement string `package require -exact NAME VERSION` builds:
/// `VERSION-VERSION` (`tclPkg.c`'s `PKG_REQUIRE` arm, which appends `"-"` and
/// the version to itself before handing the result to the ordinary
/// requirement machinery).
///
/// Modelling `-exact` as a requirement rather than a separate code path is why
/// there is exactly one satisfaction rule: the degenerate `min == max` range
/// *is* exactness.
#[must_use]
pub fn exact_requirement(version: &str) -> String {
    format!("{version}-{version}")
}

/// Which of the two best candidates `package require` takes — C Tcl's
/// `package prefer` interpreter state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PackagePrefer {
    /// Take the highest acceptable version, stable or not.
    Latest,
    /// Take the highest acceptable **stable** version, falling back to the
    /// highest acceptable version when nothing acceptable is stable.
    ///
    /// C Tcl's default (`tclsh8.6` and `tclsh9.0` both answer `stable` to a
    /// bare `package prefer`), unless `TCL_PKG_PREFER_LATEST` is set in the
    /// environment.
    #[default]
    Stable,
}

/// The index of the `package ifneeded` candidate `package require` would
/// select from `available` — a port of `SelectPackage`'s scan.
///
/// * A candidate whose version does not parse is skipped, exactly as
///   `SelectPackage` skips one `CheckVersionAndConvert` rejects.
/// * With a non-empty `requirements`, a candidate must satisfy **at least
///   one** of them (`SomeRequirementSatisfied` — the requirement list is an
///   OR, so `package require widget 1.2 2.0` accepts either range).
/// * An empty `requirements` accepts every candidate. This is the
///   *unconstrained* `package require NAME`, which still picks the best
///   version — not the first registered one.
/// * Two candidates with equal versions keep the **earlier** index, because C
///   replaces its running best only on a strictly greater version.
///
/// Returns `None` when nothing is acceptable — the `can't find package NAME …`
/// error case.
#[must_use]
pub fn select_package_version<S: AsRef<str>>(
    available: &[S],
    requirements: &[&str],
    prefer: PackagePrefer,
) -> Option<usize> {
    use core::cmp::Ordering;
    let mut best: Option<(usize, Vec<i64>)> = None;
    let mut best_stable: Option<(usize, Vec<i64>)> = None;
    for (i, candidate) in available.iter().enumerate() {
        let Some(parsed) = ParsedVersion::parse(candidate.as_ref()) else {
            continue;
        };
        if !requirements.is_empty()
            && !requirements
                .iter()
                .any(|r| satisfies_internal(&parsed.segments, r))
        {
            continue;
        }
        if best
            .as_ref()
            .is_none_or(|(_, b)| compare_internal(&parsed.segments, b).0 == Ordering::Greater)
        {
            best = Some((i, parsed.segments.clone()));
        }
        if !parsed.stable {
            continue;
        }
        if best_stable
            .as_ref()
            .is_none_or(|(_, b)| compare_internal(&parsed.segments, b).0 == Ordering::Greater)
        {
            best_stable = Some((i, parsed.segments));
        }
    }
    match prefer {
        PackagePrefer::Stable => best_stable.or(best),
        PackagePrefer::Latest => best,
    }
    .map(|(i, _)| i)
}

/// A three-valued behaviour policy, so a non-Tcl profile (`f5-bigip`) and
/// the permissive unknown-dialect fallback are **inert** — "no opinion" —
/// rather than silently defaulted to one of the real behaviours
/// (dialect-profile-model.md §11.1). Consumers short-circuit on
/// [`Ternary::Inert`] instead of guessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ternary {
    /// The behaviour applies (e.g. leading-zero integers read as octal).
    Yes,
    /// The behaviour does not apply (e.g. Tcl 9.x dropped bare-leading-zero
    /// octal, TIP 114/472).
    No,
    /// No opinion — the profile is not a Tcl runtime, or is the permissive
    /// unknown-dialect sink. Validators and const-folders abstain.
    Inert,
}

impl From<bool> for Ternary {
    /// A decided boolean as a [`Ternary`] — the inverse of
    /// [`Ternary::as_bool`], never producing [`Ternary::Inert`].
    fn from(value: bool) -> Self {
        if value { Self::Yes } else { Self::No }
    }
}

impl Ternary {
    /// The policy as an `Option<bool>`: `Inert` is `None`, so callers that
    /// already model "undecided" as `None` (the expr const-folder's octal
    /// input) consume it directly.
    #[must_use]
    pub fn as_bool(self) -> Option<bool> {
        match self {
            Self::Yes => Some(true),
            Self::No => Some(false),
            Self::Inert => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TclVersion, Ternary};

    #[test]
    fn from_dialect_maps_every_versioned_tcl() {
        // RUST_ISSUE_083: tcl9.1 must resolve to a version, not `None` (which
        // silently degrades versioned folds to the dialect-invariant subset).
        assert_eq!(
            TclVersion::from_dialect(Some("tcl8.4")),
            Some(TclVersion::V8_4)
        );
        assert_eq!(
            TclVersion::from_dialect(Some("tcl8.5")),
            Some(TclVersion::V8_5)
        );
        assert_eq!(
            TclVersion::from_dialect(Some("tcl8.6")),
            Some(TclVersion::V8_6)
        );
        assert_eq!(
            TclVersion::from_dialect(Some("tcl9.0")),
            Some(TclVersion::V9_0)
        );
        assert_eq!(
            TclVersion::from_dialect(Some("tcl9.1")),
            Some(TclVersion::V9_1)
        );
        // Unversioned / non-Tcl / unknown → None.
        assert_eq!(TclVersion::from_dialect(Some("tcl")), None);
        assert_eq!(TclVersion::from_dialect(Some("f5-irules")), None);
        assert_eq!(TclVersion::from_dialect(None), None);
    }

    #[test]
    fn v9_1_orders_at_or_above_v9_0() {
        // A `>= V9_0` gate must include 9.1.
        assert!(TclVersion::V9_1 >= TclVersion::V9_0);
        assert!(TclVersion::V9_1 > TclVersion::V8_6);
    }

    #[test]
    fn ternary_maps_inert_to_none() {
        assert_eq!(Ternary::Yes.as_bool(), Some(true));
        assert_eq!(Ternary::No.as_bool(), Some(false));
        assert_eq!(Ternary::Inert.as_bool(), None);
    }

    /// Every row pinned against a live `package vsatisfies [package provide
    /// Tcl] REQ` on `tclsh8.6` (8.6.14) and `tclsh9.0` (9.0.4).
    #[test]
    fn version_satisfies_matches_the_interpreter() {
        use super::version_satisfies;
        // Bare `X.Y` is bounded by the next *major*, not the next minor.
        assert!(version_satisfies("8.6", "8.4"));
        assert!(version_satisfies("8.6", "8.5"));
        assert!(version_satisfies("8.6", "8.6"));
        assert!(!version_satisfies("8.6", "9"));
        assert!(!version_satisfies("9.0", "8.6"));
        assert!(version_satisfies("9.0", "9"));
        assert!(version_satisfies("9.0", "9.0"));
        // Open-ended.
        assert!(version_satisfies("8.6", "8.5-"));
        assert!(version_satisfies("9.0", "8.5-"));
        assert!(!version_satisfies("8.6", "9-"));
        assert!(version_satisfies("9.0", "9-"));
        // Explicit, half-open range.
        assert!(version_satisfies("8.6", "8.5-9.0"));
        assert!(!version_satisfies("9.0", "8.5-9.0"));
        // Patch levels compare component-wise.
        assert!(version_satisfies("8.5.2", "8.5-9.0"));
    }

    /// The tcllib `pkgIndex.tcl` head guard, both ways round.
    #[test]
    fn satisfies_any_is_the_multi_requirement_or() {
        // `package vsatisfies [package provide Tcl] 8.5 9` — true on both.
        assert!(TclVersion::V8_6.satisfies_any(&["8.5", "9"]));
        assert!(TclVersion::V9_0.satisfies_any(&["8.5", "9"]));
        // …but 8.4 satisfies neither requirement.
        assert!(!TclVersion::V8_4.satisfies_any(&["8.5", "9"]));
        // An empty requirement list is never satisfied.
        assert!(!TclVersion::V9_0.satisfies_any::<&str>(&[]));
    }

    /// A requirement naming a patch level is undecidable against a
    /// `major.minor`-only release model.
    ///
    /// Oracle (`tclsh9.0`, `[package provide Tcl]` = 9.0.4):
    /// `vsatisfies 9.0.4 9.0.1` = 1, `vsatisfies 9.0.4 9.0.1-` = 1,
    /// `vsatisfies 9.0.4 9.0.9-` = 0 — two shipped 9.0 releases disagree, so
    /// the honest answer for the enum's `9.0` is neither.
    #[test]
    fn a_patch_level_requirement_is_undecidable() {
        for requirement in ["9.0.1", "9.0.1-", "9.0-9.0.2", "8.6.0"] {
            assert!(
                super::requirement_names_patch_level(requirement),
                "{requirement}"
            );
            assert_eq!(
                TclVersion::V9_0.satisfies_any_ternary(&[requirement]),
                Ternary::Inert,
                "{requirement}"
            );
        }
    }

    /// TN — every two-component requirement form still decides, and the OR
    /// short-circuits on a satisfied one beside an undecidable one.
    #[test]
    fn two_component_requirements_still_decide() {
        for requirement in ["8.5", "8.5-", "8.5-9.0", "9-", "9"] {
            assert!(
                !super::requirement_names_patch_level(requirement),
                "{requirement}"
            );
            assert_ne!(
                TclVersion::V9_0.satisfies_any_ternary(&[requirement]),
                Ternary::Inert,
                "{requirement}"
            );
        }
        assert_eq!(
            TclVersion::V9_0.satisfies_any_ternary(&["9.0.1", "9"]),
            Ternary::Yes,
        );
        assert_eq!(
            TclVersion::V8_6.satisfies_any_ternary(&["9.0.1", "9"]),
            Ternary::Inert,
        );
        assert_eq!(TclVersion::V8_6.satisfies_any_ternary(&["9"]), Ternary::No);
    }

    /// Per-rule TP/FP/TN/FN for the four requirement forms, issue #1090.
    ///
    /// The whole grid lives in the pinned corpus
    /// (`tests/data/package_version_oracle.txt`); these are the rows that name
    /// the rule they exercise, so a regression reads as "the min-bound rule
    /// broke" rather than "142 corpus pairs disagree".
    #[test]
    fn each_requirement_form_decides_its_own_way() {
        use super::version_satisfies as sat;
        // Min bound within the major: `[1.2, 2)`.
        assert!(sat("1.2", "1.2"), "TP — the bound itself");
        assert!(sat("1.10", "1.2"), "TP — 10 is a number, not a string tail");
        assert!(sat("1.2.3", "1.2"), "TP — a patch release of the bound");
        assert!(!sat("1.1", "1.2"), "TN — below the bound");
        assert!(!sat("2.0", "1.2"), "FP guard — the next major is excluded");
        assert!(!sat("0.9", "1.2"), "TN — a lower major");
        // …and the alpha of the bound *is* accepted (the `a0` pad), which is
        // the rule a naive `>=` comparison gets wrong in the FN direction.
        assert!(sat("1.2a1", "1.2"), "TP — `vsatisfies 1.2a1 1.2` is 1");
        assert!(!sat("1.2a1", "1.2.0"), "TN — but not of a longer bound");

        // Open-ended `min-`: no upper bound at all, so the next major counts.
        assert!(sat("2.0", "1.2-"), "TP — no major cap");
        assert!(sat("1.2a1", "1.2-"), "TP — the alpha pad applies here too");
        assert!(!sat("1.1", "1.2-"), "TN — still bounded below");

        // Half-open range `min-max`.
        assert!(sat("1.9", "1.2-2.0"), "TP — inside");
        assert!(!sat("2.0", "1.2-2.0"), "FP guard — the max is excluded");
        assert!(!sat("1.1", "1.2-2.0"), "TN — below the min");
        assert!(!sat("1.3", "1.2a1-1.3"), "FP guard — max excluded, padded");

        // Degenerate `v-v` — what `-exact` builds.
        assert!(sat("2.0", "2.0-2.0"), "TP");
        assert!(
            sat("2.0.0", "2.0-2.0"),
            "TP — trailing zeros are not a digit"
        );
        assert!(!sat("2.0a1", "2.0-2.0"), "FP guard — no alpha pad on exact");
        assert!(!sat("2.3", "2.0-2.0"), "TN — a later release");

        // Malformed: real Tcl raises, so nothing satisfies.
        for bad in ["1.2-1.3-1.4", "-1.2", "", "a-b", "1.2-x"] {
            assert!(!sat("1.2", bad), "malformed requirement {bad:?}");
        }
        for bad in ["", "1.", ".1", "1..2", "1a", "1a1b2", " 1.2"] {
            assert!(!sat(bad, "1.2"), "malformed version {bad:?}");
        }
    }

    /// `select_package_version`'s two selection axes, issue #1090: highest
    /// acceptable version, and `package prefer`'s stable-first tie-break.
    #[test]
    fn selection_takes_the_highest_acceptable_preferring_stable() {
        use super::{PackagePrefer, select_package_version as pick};
        let avail = ["1.5", "2.3", "2.0"];
        // TP — unconstrained picks the best, not index 0.
        assert_eq!(pick(&avail, &[], PackagePrefer::Stable), Some(1));
        // TP — a constraint narrows, then the best of what is left wins.
        assert_eq!(pick(&avail, &["2.0"], PackagePrefer::Stable), Some(1));
        assert_eq!(pick(&avail, &["1.2"], PackagePrefer::Stable), Some(0));
        // TP — the requirement list is an OR.
        assert_eq!(
            pick(&avail, &["1.2", "2.0"], PackagePrefer::Stable),
            Some(1)
        );
        // TN — nothing acceptable, and an empty candidate list.
        assert_eq!(pick(&avail, &["3.0"], PackagePrefer::Stable), None);
        assert_eq!(pick::<&str>(&[], &[], PackagePrefer::Stable), None);
        // TN — a candidate whose version does not parse is skipped, not
        // treated as version 0.
        assert_eq!(
            pick(&["not-a-version", "1.0"], &[], PackagePrefer::Stable),
            Some(1)
        );

        // Prefer: the prerelease is the higher version but loses by default.
        let mixed = ["1.2", "1.3b1"];
        assert_eq!(pick(&mixed, &[], PackagePrefer::Stable), Some(0));
        assert_eq!(pick(&mixed, &[], PackagePrefer::Latest), Some(1));
        // FN guard — with nothing stable to prefer, the prerelease still wins
        // under `stable`; abstaining there would resolve nothing at all.
        assert_eq!(pick(&["1.3b1"], &[], PackagePrefer::Stable), Some(0));
        // Ties keep the earlier candidate, so discovery order breaks them.
        assert_eq!(pick(&["1.0", "1.0.0"], &[], PackagePrefer::Stable), Some(0));
    }

    #[test]
    fn version_string_round_trips_through_from_package_version() {
        for v in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            assert_eq!(
                TclVersion::from_package_version(v.version_string()),
                Some(v)
            );
        }
    }
}
