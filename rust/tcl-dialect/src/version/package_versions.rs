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

//! Package versions as C Tcl reads them: the internal representation, validation, comparison, requirement satisfaction and provider selection.

use super::TclVersion;
use super::engine_policies::Ternary;

impl TclVersion {
    /// Does this release **definitely** satisfy any of `requirements` — the
    /// answer `package vsatisfies [package provide Tcl] REQ ?REQ …?` gives on
    /// every build of the release line?
    ///
    /// The decided half of [`Self::satisfies_any_ternary`]: a requirement the
    /// line's builds disagree about ([`Ternary::Inert`]) is `false` here, so
    /// this never claims more than the ternary does.  A caller that needs to
    /// tell "no" from "cannot say" — every consumer that abstains — must ask
    /// the ternary.
    ///
    /// An empty `requirements` list is `false`: real `package vsatisfies`
    /// rejects it as a wrong-argument-count error, and no caller here has a
    /// meaningful "satisfies nothing" question to ask.
    #[must_use]
    pub fn satisfies_any<S: AsRef<str>>(self, requirements: &[S]) -> bool {
        self.satisfies_any_ternary(requirements) == Ternary::Yes
    }

    /// [`Self::satisfies_any`], but [`Ternary::Inert`] when the answer really
    /// does depend on a **patch level** this enum does not model.
    ///
    /// [`TclVersion`] names a release *line*, not a build: `V9_0` stands for
    /// every `9.0.x` the user might be running.  So the honest question is not
    /// "does `9.0` satisfy this requirement" but "does **every** member of the
    /// line satisfy it, **none** of them, or some" — a requirement's satisfying
    /// set is a version interval (`package(n)`: `min` … `max`), and so is a
    /// release line, so the three answers are decided by two endpoint tests
    /// plus one containment test:
    ///
    /// | line vs requirement interval | answer |
    /// |---|---|
    /// | line ⊆ requirement | [`Ternary::Yes`] |
    /// | line ∩ requirement = ∅ | [`Ternary::No`] |
    /// | otherwise | [`Ternary::Inert`] |
    ///
    /// This replaces the earlier "any bound with three components is
    /// undecidable" rule, which abstained on requirements the line's *major*
    /// alone settles.  Both directions gained precision, and both are
    /// oracle-checked — `package vsatisfies` takes the candidate version as an
    /// argument, so one interpreter answers for every line, and
    /// `tests/data/package_version_oracle.txt` pins the operator itself
    /// byte-identical on 8.6.14 and 9.0.4:
    ///
    // tclsh-proof: `package vsatisfies 8.6.14 9.0.1` → 0, and so does every
    // other 8.6.x — no member of the 8.6 line can reach a 9.0 bound.
    /// * `9.0.1` against `V8_6` was `Inert`, is now `No` — no 8.6.x reaches a
    ///   9.0 bound however the patch level lands.
    // tclsh-proof: `package vsatisfies 8.6.14 8.6-8.6` → 0 while
    // `package vsatisfies 8.6 8.6-8.6` → 1: the degenerate (`-exact`) range
    // accepts only the bound itself, so a patch release of the line fails it.
    /// * `8.6-8.6` (what `package require -exact 8.6` builds) against `V8_6`
    ///   was `Yes`, is now `Inert` — `-exact 8.6` accepts `8.6`/`8.6.0` and
    ///   rejects `8.6.14`, so which it is depends on the running build.
    ///
    /// The OR short-circuits exactly as `package vsatisfies` does: a
    /// requirement the whole line satisfies settles the test [`Ternary::Yes`]
    /// however many undecidable ones sit beside it, and `No` is only reached
    /// when every requirement is refused by every member of the line.
    ///
    /// An empty `requirements` list is [`Ternary::No`], matching
    /// [`Self::satisfies_any`].
    #[must_use]
    pub fn satisfies_any_ternary<S: AsRef<str>>(self, requirements: &[S]) -> Ternary {
        let mut undecidable = false;
        for requirement in requirements {
            match self.satisfies_ternary(requirement.as_ref()) {
                Ternary::Yes => return Ternary::Yes,
                Ternary::Inert => undecidable = true,
                Ternary::No => {}
            }
        }
        if undecidable {
            Ternary::Inert
        } else {
            Ternary::No
        }
    }

    /// One requirement of [`Self::satisfies_any_ternary`], evaluated over the
    /// whole release line rather than against a single version string.
    ///
    /// The line is the closed interval `[M.m, M.m.PATCH_CEILING]`
    /// ([`PATCH_CEILING`]); `M.m` and `M.m.0` are the same version to the
    /// comparator (trailing zero components are not significant), so the low
    /// endpoint is the line's first release.
    ///
    /// A requirement's satisfying set is an interval too, which is what makes
    /// two endpoint tests sufficient for the `Yes` / `Inert` split: if both
    /// ends of the line satisfy it, so does everything between them.  When
    /// *neither* end does, the requirement interval is either entirely outside
    /// the line or strictly inside it, and the discriminator is whether the
    /// requirement's own **min bound** — always the first version its interval
    /// admits — falls within the line.
    ///
    /// A malformed requirement is [`Ternary::No`], the same conservative
    /// reading [`version_satisfies`] takes (real Tcl raises instead).
    fn satisfies_ternary(self, requirement: &str) -> Ternary {
        let low = self.version_string();
        let high = format!("{low}.{PATCH_CEILING}");
        match (
            version_satisfies(low, requirement),
            version_satisfies(&high, requirement),
        ) {
            (true, true) => Ternary::Yes,
            (false, false) => {
                let min = requirement
                    .split_once('-')
                    .map_or(requirement, |(min, _)| min);
                if ParsedVersion::parse(min).is_some()
                    && compare_versions(min, low) != core::cmp::Ordering::Less
                    && compare_versions(min, &high) != core::cmp::Ordering::Greater
                {
                    // The requirement admits only versions inside this line —
                    // some patch releases satisfy it, the endpoints do not.
                    Ternary::Inert
                } else {
                    Ternary::No
                }
            }
            _ => Ternary::Inert,
        }
    }
}

/// The patch component standing in for "the last release this `major.minor`
/// line will ever carry", the upper endpoint
/// [`TclVersion::satisfies_ternary`] tests.
///
/// Version components compare by digit count first and are never parsed into a
/// fixed integer width ([`Segment::cmp`]), so a twenty-digit component is
/// simply larger than every component any requirement or release realistically
/// writes — including the deeper components of a four-part version
/// (`8.6.16.2` orders below `8.6.<ceiling>` at the third component).
const PATCH_CEILING: &str = "99999999999999999999";

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

/// One element of a version's internal representation.
///
/// `CheckVersionAndConvert` rewrites a version into a space-separated list of
/// numbers in which every separator becomes a number of its own: `.` → `0`,
/// `a` → `-2`, `b` → `-1`.  Encoding the separator as a *number* is what makes
/// an alpha/beta release order below the dotted patch release at the same
/// position without a second comparison rule.
///
/// The numeric components are kept as **digit strings**, not integers, because
/// `CompareVersions` does not compute a numeric value either — its comment
/// says so outright:
///
/// > Rewritten to not compute a numeric value for the extracted version
/// > number, but do string comparison. Skip any leading zeros for that to
/// > work. This change breaks through the 32bit-limit on version numbers.
///
/// So a component of any length compares exactly: `package vcompare
/// 9223372036854775807 9223372036854775808` is `-1` on both interpreters, and
/// a forty-digit component still orders correctly.  Parsing into any fixed
/// integer width would collapse everything past that width into one value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Segment<'v> {
    /// The `a` (alpha) separator — C's `-2`, below every other segment.
    Alpha,
    /// The `b` (beta) separator — C's `-1`.
    Beta,
    /// A numeric component, **leading zeros already stripped**, so `""` is
    /// zero and `"0005"` is held as `"5"`.  C strips them by advancing past
    /// `'0'` before measuring the run, which is also why a literal `0`
    /// component and a component that simply ran out compare equal.
    Number(&'v [u8]),
}

/// Zero — the value a version that has run out of components compares as.
///
/// C reaches the same state by running off the end of the shorter string: the
/// leading-zero skip leaves an empty run, which then ties with a literal `0`,
/// sorts below any longer digit run, and sorts above the negative separator
/// markers via the sign shortcut.
const ZERO: Segment<'static> = Segment::Number(b"");

impl Segment<'_> {
    /// The `CompareVersions` per-segment rule.
    ///
    /// * A separator marker is negative in C, so it loses against any number
    ///   (the sign shortcut), and `a` (`-2`) loses to `b` (`-1`) — C compares
    ///   the two magnitudes and flips the result.
    /// * Two numbers compare by **digit count first**, then lexicographically
    ///   ("shorter string is smaller number", then `strcmp` on equal lengths).
    ///   With leading zeros stripped that is exact numeric ordering at any
    ///   width.
    pub(super) fn cmp(self, other: Self) -> core::cmp::Ordering {
        use core::cmp::Ordering;
        match (self, other) {
            (Self::Alpha, Self::Alpha) | (Self::Beta, Self::Beta) => Ordering::Equal,
            (Self::Alpha, _) | (Self::Beta, Self::Number(_)) => Ordering::Less,
            (_, Self::Alpha) | (Self::Number(_), Self::Beta) => Ordering::Greater,
            (Self::Number(a), Self::Number(b)) => a.len().cmp(&b.len()).then_with(|| a.cmp(b)),
        }
    }
}

/// Parsed segments held inline: registry lookups check a version against
/// every authored row's window, so these parses are by far the most frequent
/// and almost always short. A longer version spills to the heap.
// Inline by design: the point is a stack-only parse.
#[allow(clippy::large_enum_variant)]
#[derive(Clone)]
enum SegmentBuf<'v> {
    Inline([Segment<'v>; SEGMENT_INLINE], u8),
    Heap(Vec<Segment<'v>>),
}

const SEGMENT_INLINE: usize = 16;

impl<'v> SegmentBuf<'v> {
    pub(super) const fn new() -> Self {
        Self::Inline([ZERO; SEGMENT_INLINE], 0)
    }

    pub(super) fn push(&mut self, segment: Segment<'v>) {
        match self {
            Self::Inline(items, len) if usize::from(*len) < SEGMENT_INLINE => {
                items[usize::from(*len)] = segment;
                *len += 1;
            }
            Self::Inline(items, len) => {
                let mut spilled = items[..usize::from(*len)].to_vec();
                spilled.push(segment);
                *self = Self::Heap(spilled);
            }
            Self::Heap(items) => items.push(segment),
        }
    }
}

impl<'v> core::ops::Deref for SegmentBuf<'v> {
    type Target = [Segment<'v>];
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Inline(items, len) => &items[..usize::from(*len)],
            Self::Heap(items) => items,
        }
    }
}

impl PartialEq for SegmentBuf<'_> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl Eq for SegmentBuf<'_> {}

impl core::fmt::Debug for SegmentBuf<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

/// A version in C Tcl's internal representation, plus its stability flag.
///
/// `1.2` is `[1, 0, 2]`, `1.2a1` is `[1, 0, 2, a, 1]`, `1.2.3` is
/// `[1, 0, 2, 0, 3]` — the `0`s being the segments the `.` separators inject.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedVersion<'v> {
    /// The internal-rep segments.
    segments: SegmentBuf<'v>,
    /// `false` when an `a` or `b` separator occurred — C Tcl's `hasunstable`,
    /// the flag `SelectPackage` uses to maintain its "best stable" candidate.
    stable: bool,
}

impl<'v> ParsedVersion<'v> {
    /// Validate and convert `string`, or `None` when it is not a well-formed
    /// version — a port of `CheckVersionAndConvert`'s TIP 268 rules:
    ///
    /// 1. the first character must be a digit;
    /// 2. every other character must be a digit, `.`, `a`, or `b`;
    /// 3. only one of `a` / `b` may occur;
    /// 4. neither `a`, `b`, nor `.` may sit next to a `.`;
    /// 5. the last character may not be a separator.
    ///
    /// When enabled by a Tcl 9 release policy, the C checker stops at a `+`
    /// suffix. The suffix remains part of the package's recorded spelling,
    /// but comparison and satisfaction use the version prefix before it.
    ///
    /// Real Tcl raises `expected version number but got "…"` where this
    /// answers `None`; every caller here turns that into the conservative
    /// static answer (unsatisfiable / unselectable) rather than a panic.
    pub(super) fn parse(string: &'v str) -> Option<Self> {
        Self::parse_for(string, false)
    }

    fn parse_for(string: &'v str, allow_plus_suffix: bool) -> Option<Self> {
        Self::parse_bytes_for(string.as_bytes(), allow_plus_suffix)
    }

    fn parse_bytes_for(string: &'v [u8], allow_plus_suffix: bool) -> Option<Self> {
        let bytes = package_version_prefix_bytes(string, allow_plus_suffix);
        if !bytes.first().is_some_and(u8::is_ascii_digit) {
            return None;
        }
        let mut segments = SegmentBuf::new();
        let mut run_start = 0usize;
        let mut has_unstable = false;
        let mut prev = bytes[0];
        for (i, &c) in bytes.iter().enumerate().skip(1) {
            if c.is_ascii_digit() {
                prev = c;
                continue;
            }
            let separator = match c {
                b'.' => ZERO,
                b'a' => Segment::Alpha,
                b'b' => Segment::Beta,
                _ => return None,
            };
            // Rule 3 — a second `a`/`b`; rule 4 — a separator adjacent to a
            // `.`, or a `.` adjacent to any separator.
            if (has_unstable && c != b'.')
                || (matches!(prev, b'a' | b'b') && c == b'.')
                || prev == b'.'
            {
                return None;
            }
            has_unstable |= c != b'.';
            segments.push(number_segment(&bytes[run_start..i]));
            segments.push(separator);
            run_start = i + 1;
            prev = c;
        }
        // Rule 5 — a trailing separator.
        if matches!(prev, b'.' | b'a' | b'b') {
            return None;
        }
        segments.push(number_segment(&bytes[run_start..]));
        Some(Self {
            segments,
            stable: !has_unstable,
        })
    }

    /// A best-effort rep for a string [`Self::parse`] rejects, so the *total*
    /// [`compare_versions`] never has to invent an answer out of nothing: digit
    /// runs become segments, recognised separators become their markers, and
    /// any other character is skipped. Never used to decide satisfaction —
    /// only to order two strings that are not versions in the first place.
    fn lenient_bytes_for(string: &'v [u8], allow_plus_suffix: bool) -> SegmentBuf<'v> {
        let bytes = package_version_prefix_bytes(string, allow_plus_suffix);
        let mut segments = SegmentBuf::new();
        let mut run_start = 0usize;
        for (i, &c) in bytes.iter().enumerate() {
            if c.is_ascii_digit() {
                continue;
            }
            let separator = match c {
                b'.' => ZERO,
                b'a' => Segment::Alpha,
                b'b' => Segment::Beta,
                _ => {
                    // Skipped entirely: close the run before it and reopen
                    // after, so `1x2` still reads as two components.
                    segments.push(number_segment(&bytes[run_start..i]));
                    run_start = i + 1;
                    continue;
                }
            };
            segments.push(number_segment(&bytes[run_start..i]));
            segments.push(separator);
            run_start = i + 1;
        }
        segments.push(number_segment(&bytes[run_start..]));
        segments
    }
}

/// A `package` diagnostic whose `errorCode` is release policy
/// ([`TclVersion::package_error_code`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageDiagnostic {
    /// An `ifneeded` or `package unknown` script ended with a non-error,
    /// non-ok code.
    BadResult,
    /// A loader provided a different version than the one it was selected for.
    WrongProvide,
    /// A loader returned without providing the package.
    Unprovided,
    /// A loader required the package it is providing.
    Circularity,
    /// The provided version does not satisfy the request, or a second
    /// `package provide` names another version.
    VersionConflict,
    /// No provider satisfies `package require`.
    Unfound,
    /// A word is not a valid version number.
    InvalidVersion,
    /// A requirement has more than one `-`.
    InvalidVersionRange,
    /// `package present` found nothing provided; the code words are followed
    /// by the package name.
    NotPresent,
}

impl PackageDiagnostic {
    pub(super) const fn error_code_words(self) -> &'static [&'static str] {
        match self {
            Self::BadResult => &["TCL", "PACKAGE", "BADRESULT"],
            Self::WrongProvide => &["TCL", "PACKAGE", "WRONGPROVIDE"],
            Self::Unprovided => &["TCL", "PACKAGE", "UNPROVIDED"],
            Self::Circularity => &["TCL", "PACKAGE", "CIRCULARITY"],
            Self::VersionConflict => &["TCL", "PACKAGE", "VERSIONCONFLICT"],
            Self::Unfound => &["TCL", "PACKAGE", "UNFOUND"],
            Self::InvalidVersion => &["TCL", "VALUE", "VERSION"],
            Self::InvalidVersionRange => &["TCL", "VALUE", "VERSIONRANGE"],
            Self::NotPresent => &["TCL", "LOOKUP", "PACKAGE"],
        }
    }
}

/// The reason a package requirement failed Tcl's version grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequirementValidationError<'v> {
    /// A bound was not a valid package version; the borrowed value is the
    /// exact text Tcl reports.
    InvalidVersion(&'v str),
    /// More than one `-` appeared in the requirement; the borrowed value is
    /// the exact text Tcl reports.
    InvalidRange(&'v str),
}

/// Failure in the package requirement grammar, retaining the original bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequirementBytesValidationError<'v> {
    /// Invalid single version or range endpoint.
    InvalidVersion(&'v [u8]),
    /// Invalid range with more than one separator.
    InvalidRange(&'v [u8]),
}

/// Validate an already selected package requirement byte extent.
///
/// # Errors
/// The exact single version, endpoint, or whole range used by the diagnostic.
pub fn validate_requirement_bytes_for(
    requirement: &[u8],
    release: TclVersion,
) -> Result<(), RequirementBytesValidationError<'_>> {
    let allow_suffix = release.allows_package_version_suffix();
    if !release.has_package_requirements() || (allow_suffix && requirement.contains(&b'+')) {
        return validate_version_bytes_for(requirement, release)
            .then_some(())
            .ok_or(RequirementBytesValidationError::InvalidVersion(requirement));
    }
    let Some(dash) = requirement.iter().position(|byte| *byte == b'-') else {
        return validate_version_bytes_for(requirement, release)
            .then_some(())
            .ok_or(RequirementBytesValidationError::InvalidVersion(requirement));
    };
    if requirement[dash + 1..].contains(&b'-') {
        return Err(RequirementBytesValidationError::InvalidRange(requirement));
    }
    let minimum = &requirement[..dash];
    let maximum = &requirement[dash + 1..];
    if !validate_version_bytes_for(minimum, release) {
        return Err(RequirementBytesValidationError::InvalidVersion(minimum));
    }
    if !maximum.is_empty() && !validate_version_bytes_for(maximum, release) {
        return Err(RequirementBytesValidationError::InvalidVersion(maximum));
    }
    Ok(())
}

/// Check one package version using the strict, release-agnostic parser.
/// Runtime commands for a pinned interpreter use [`validate_version_for`].
#[must_use]
pub fn validate_version(version: &str) -> bool {
    validate_version_for(version, TclVersion::V8_6)
}

/// Check one package version using a pinned Tcl release's grammar.
#[must_use]
pub fn validate_version_for(version: &str, release: TclVersion) -> bool {
    validate_version_bytes_for(version.as_bytes(), release)
}

/// Byte-preserving package semantics for already selected input extents.
#[must_use]
pub fn validate_version_bytes_for(version: &[u8], release: TclVersion) -> bool {
    (release.has_package_requirements()
        || version
            .iter()
            .copied()
            .all(|byte| byte.is_ascii_digit() || byte == b'.'))
        && ParsedVersion::parse_bytes_for(version, release.allows_package_version_suffix())
            .is_some()
}

/// Check one package requirement using the strict, release-agnostic parser.
/// An empty upper bound is valid (`min-`); a second dash is a range error.
pub fn validate_requirement(requirement: &str) -> Result<(), RequirementValidationError<'_>> {
    validate_requirement_for(requirement, TclVersion::V8_6)
}

/// Check one package requirement using a pinned Tcl release's grammar.
pub fn validate_requirement_for(
    requirement: &str,
    release: TclVersion,
) -> Result<(), RequirementValidationError<'_>> {
    validate_requirement_bytes_for(requirement.as_bytes(), release).map_err(|error| match error {
        RequirementBytesValidationError::InvalidVersion(bytes) => {
            RequirementValidationError::InvalidVersion(
                std::str::from_utf8(bytes).expect("original Unicode endpoint"),
            )
        }
        RequirementBytesValidationError::InvalidRange(bytes) => {
            RequirementValidationError::InvalidRange(
                std::str::from_utf8(bytes).expect("original Unicode range"),
            )
        }
    })
}

/// The written endpoints of a validated Tcl requirement.
#[must_use]
pub fn package_requirement_bounds_for(
    requirement: &str,
    release: TclVersion,
) -> Option<(&str, Option<&str>)> {
    validate_requirement_for(requirement, release).ok()?;
    if release.has_package_requirements()
        && let Some((lower, upper)) = requirement.split_once('-')
    {
        return Some((lower, (!upper.is_empty()).then_some(upper)));
    }
    Some((requirement, None))
}

/// Comparable portion of a Tcl package version, without release suffix data.
#[must_use]
pub fn package_version_prefix_for(version: &str, release: TclVersion) -> &str {
    let size =
        package_version_prefix_bytes(version.as_bytes(), release.allows_package_version_suffix())
            .len();
    &version[..size]
}

fn package_version_prefix_bytes(version: &[u8], allow_suffix: bool) -> &[u8] {
    if allow_suffix && let Some(end) = version.iter().position(|byte| *byte == b'+') {
        return &version[..end];
    }
    version
}

/// A digit run as a [`Segment::Number`], with C's leading-zero skip applied so
/// `"0005"`, `"5"` compare equal and `"000"`, `"0"`, `""` all read as zero.
fn number_segment(run: &[u8]) -> Segment<'_> {
    let first = run
        .iter()
        .position(|byte| *byte != b'0')
        .unwrap_or(run.len());
    Segment::Number(&run[first..])
}

/// Compare two internal reps, returning the ordering plus whether the deciding
/// difference was in the **first** segment (C Tcl's `isMajorPtr` out-param).
///
/// A rep shorter than the other is padded with [`ZERO`] — the exact effect of
/// the C loop running off the end of the shorter string.
fn compare_internal(a: &[Segment<'_>], b: &[Segment<'_>]) -> (core::cmp::Ordering, bool) {
    use core::cmp::Ordering;
    for i in 0..a.len().max(b.len()) {
        let x = a.get(i).copied().unwrap_or(ZERO);
        let y = b.get(i).copied().unwrap_or(ZERO);
        match x.cmp(y) {
            Ordering::Equal => {}
            other => return (other, i == 0),
        }
    }
    (Ordering::Equal, false)
}

/// Compare two strict package version numbers as `package vcompare` does.
/// A pinned runtime uses [`compare_versions_for`] when its release accepts
/// Tcl 9 `+` suffixes.
///
/// Trailing zero components are *not* significant (`1.2` == `1.2.0`), leading
/// zeros are not either (`0005` == `5`), and an alpha/beta release orders below
/// the release it leads up to but above the previous one (`1.2` < `1.2b1` is
/// false, `1.2` < `1.3b1` is true).  Numeric components compare exactly at any
/// width — `9223372036854775807` < `9223372036854775808` — because they are
/// never parsed into a fixed integer type.  All of it is pinned against
/// `tclsh8.6` and `tclsh9.0` in
/// `tcl-dialect/tests/package_version_oracle.rs`.
///
/// Total: a string that is not a well-formed version is compared through a
/// best-effort rep rather than raising, because callers order provider lists
/// that a recorder has already filtered and have no error channel.
#[must_use]
pub fn compare_versions(a: &str, b: &str) -> core::cmp::Ordering {
    compare_versions_for(a, b, TclVersion::V8_6)
}

/// Compare package versions using a pinned Tcl release's grammar.
#[must_use]
pub fn compare_versions_for(a: &str, b: &str, release: TclVersion) -> core::cmp::Ordering {
    compare_versions_bytes_for(a.as_bytes(), b.as_bytes(), release)
}

/// A version's comparison segments, parsed once and owned, ordered exactly
/// as [`compare_versions`] orders the spelling it was built from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionOrderKey(Box<[OwnedSegment]>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum OwnedSegment {
    Alpha,
    Beta,
    Number(Box<[u8]>),
}

impl OwnedSegment {
    fn view(&self) -> Segment<'_> {
        match self {
            Self::Alpha => Segment::Alpha,
            Self::Beta => Segment::Beta,
            Self::Number(digits) => Segment::Number(digits),
        }
    }
}

impl VersionOrderKey {
    /// The key [`compare_versions`] compares `version` by.
    #[must_use]
    pub fn new(version: &str) -> Self {
        let bytes = version.as_bytes();
        let allow_plus_suffix = TclVersion::V8_6.allows_package_version_suffix();
        let segments = ParsedVersion::parse_bytes_for(bytes, allow_plus_suffix).map_or_else(
            || ParsedVersion::lenient_bytes_for(bytes, allow_plus_suffix),
            |parsed| parsed.segments,
        );
        Self(
            segments
                .iter()
                .map(|segment| match *segment {
                    Segment::Alpha => OwnedSegment::Alpha,
                    Segment::Beta => OwnedSegment::Beta,
                    Segment::Number(digits) => OwnedSegment::Number(digits.into()),
                })
                .collect(),
        )
    }
}

impl PartialOrd for VersionOrderKey {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VersionOrderKey {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        for i in 0..self.0.len().max(other.0.len()) {
            let x = self.0.get(i).map_or(ZERO, OwnedSegment::view);
            let y = other.0.get(i).map_or(ZERO, OwnedSegment::view);
            match x.cmp(y) {
                core::cmp::Ordering::Equal => {}
                order => return order,
            }
        }
        core::cmp::Ordering::Equal
    }
}

/// Byte-preserving package semantics for already selected input extents.
#[must_use]
pub fn compare_versions_bytes_for(a: &[u8], b: &[u8], release: TclVersion) -> core::cmp::Ordering {
    let allow_plus_suffix = release.allows_package_version_suffix();
    let va = ParsedVersion::parse_bytes_for(a, allow_plus_suffix).map_or_else(
        || ParsedVersion::lenient_bytes_for(a, allow_plus_suffix),
        |p| p.segments,
    );
    let vb = ParsedVersion::parse_bytes_for(b, allow_plus_suffix).map_or_else(
        || ParsedVersion::lenient_bytes_for(b, allow_plus_suffix),
        |p| p.segments,
    );
    compare_internal(&va, &vb).0
}

/// Whether `version` is a **stable** release — i.e. carries no `a`/`b`
/// (alpha/beta) separator. An unparseable version is not stable.
///
/// This is C Tcl's `hasunstable` flag, the input to `package prefer`'s choice
/// between the best and the best-stable candidate.
#[must_use]
pub fn version_is_stable(version: &str) -> bool {
    version_is_stable_for(version, TclVersion::V8_6)
}

/// Check package-version stability using a pinned Tcl release's grammar.
#[must_use]
pub fn version_is_stable_for(version: &str, release: TclVersion) -> bool {
    version_is_stable_bytes_for(version.as_bytes(), release)
}

/// Byte-preserving package semantics for already selected input extents.
#[must_use]
pub fn version_is_stable_bytes_for(version: &[u8], release: TclVersion) -> bool {
    validate_version_bytes_for(version, release)
        && ParsedVersion::parse_bytes_for(version, release.allows_package_version_suffix())
            .is_some_and(|p| p.stable)
}

/// Does the concrete strict `version` satisfy one `package vsatisfies`
/// requirement? A pinned runtime uses [`version_satisfies_for`] when its
/// release accepts Tcl 9 `+` suffixes.
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
    version_satisfies_for(version, requirement, TclVersion::V8_6)
}

/// Check package-version satisfaction using a pinned Tcl release's grammar.
#[must_use]
pub fn version_satisfies_for(version: &str, requirement: &str, release: TclVersion) -> bool {
    version_satisfies_bytes_for(version.as_bytes(), requirement.as_bytes(), release)
}

/// Byte-preserving package semantics for already selected input extents.
#[must_use]
pub fn version_satisfies_bytes_for(
    version: &[u8],
    requirement: &[u8],
    release: TclVersion,
) -> bool {
    if !validate_version_bytes_for(version, release)
        || validate_requirement_bytes_for(requirement, release).is_err()
    {
        return false;
    }
    let allow_plus_suffix = release.allows_package_version_suffix();
    let Some(have) = ParsedVersion::parse_bytes_for(version, allow_plus_suffix) else {
        return false;
    };
    satisfies_internal(&have.segments, requirement, allow_plus_suffix)
}

/// Check exact package-version equality using a pinned Tcl release's grammar.
/// This keeps `package require -exact` from encoding a version containing a
/// Tcl 9 `+` suffix as an ambiguous textual range.
#[must_use]
pub fn version_matches_exact_for(version: &str, requested: &str, release: TclVersion) -> bool {
    version_matches_exact_bytes_for(version.as_bytes(), requested.as_bytes(), release)
}

/// Byte-preserving package semantics for already selected input extents.
#[must_use]
pub fn version_matches_exact_bytes_for(
    version: &[u8],
    requested: &[u8],
    release: TclVersion,
) -> bool {
    if !validate_version_bytes_for(version, release)
        || !validate_version_bytes_for(requested, release)
    {
        return false;
    }
    let allow_plus_suffix = release.allows_package_version_suffix();
    let Some(version) = ParsedVersion::parse_bytes_for(version, allow_plus_suffix) else {
        return false;
    };
    let Some(requested) = ParsedVersion::parse_bytes_for(requested, allow_plus_suffix) else {
        return false;
    };
    compare_internal(&version.segments, &requested.segments).0 == core::cmp::Ordering::Equal
}

/// [`version_satisfies`] against an already-parsed candidate — the form
/// [`select_package_version`] needs so a candidate is converted once for the
/// whole requirement list.
fn satisfies_internal(have: &[Segment<'_>], requirement: &[u8], allow_plus_suffix: bool) -> bool {
    let Some((lo, hi)) = requirement
        .iter()
        .position(|byte| *byte == b'-')
        .map(|at| (&requirement[..at], &requirement[at + 1..]))
    else {
        return satisfies_bare(have, requirement, allow_plus_suffix);
    };
    // Tcl 8 checks the whole range before splitting it. Tcl 9's version
    // conversion stops at `+` for each endpoint, so dashes after a suffix
    // belong to that ignored suffix rather than making the upper endpoint a
    // second range.
    if !allow_plus_suffix && hi.contains(&b'-') {
        return false;
    }
    satisfies_range_with_policy(have, lo, (!hi.is_empty()).then_some(hi), allow_plus_suffix)
}

/// The no-dash arm: a simple version. The requirement is padded with an alpha
/// segment, and the candidate must be equal or greater *without* the
/// difference landing in the major component — which is what bounds a bare
/// `X.Y` at the next major without naming an upper bound.
fn satisfies_bare(have: &[Segment<'_>], requirement: &[u8], allow_plus_suffix: bool) -> bool {
    use core::cmp::Ordering;
    let Some(mut req) =
        ParsedVersion::parse_bytes_for(requirement, allow_plus_suffix).map(|p| p.segments)
    else {
        return false;
    };
    req.push(Segment::Alpha);
    let (ord, is_major) = compare_internal(have, &req);
    ord == Ordering::Equal || (ord == Ordering::Greater && !is_major)
}

/// The `min-max` arm, taking the two bounds rather than the string that
/// spells them — so a caller holding a window already split in two
/// ([`version_in_any_window`]) does not have to `format!` it back together
/// only for this to split it again.
fn satisfies_range(have: &[Segment<'_>], lo: &str, hi: Option<&str>) -> bool {
    satisfies_range_with_policy(have, lo.as_bytes(), hi.map(str::as_bytes), false)
}

fn satisfies_range_with_policy(
    have: &[Segment<'_>],
    lo: &[u8],
    hi: Option<&[u8]>,
    allow_plus_suffix: bool,
) -> bool {
    use core::cmp::Ordering;
    let Some(min) = ParsedVersion::parse_bytes_for(lo, allow_plus_suffix).map(|p| p.segments)
    else {
        return false;
    };
    let Some(hi) = hi else {
        // `min-` — open-ended above.
        let mut min = min;
        min.push(Segment::Alpha);
        return compare_internal(have, &min).0 != Ordering::Less;
    };
    let Some(max) = ParsedVersion::parse_bytes_for(hi, allow_plus_suffix).map(|p| p.segments)
    else {
        return false;
    };
    if compare_internal(&min, &max).0 == Ordering::Equal {
        // `v-v` — the exact form. Compared unpadded, so an alpha/beta release
        // of `v` does *not* satisfy it.
        return compare_internal(&min, have).0 == Ordering::Equal;
    }
    let (mut min, mut max) = (min, max);
    min.push(Segment::Alpha);
    max.push(Segment::Alpha);
    compare_internal(&min, have).0 != Ordering::Greater
        && compare_internal(have, &max).0 == Ordering::Less
}

/// Whether `version` falls in any of the half-open `windows`, each given as
/// its `(from, until)` bounds — [`version_satisfies`] against `"from-until"`
/// (or `"from-"` for an open window), without spelling that requirement out.
///
/// Parse the candidate once and compare each retained bound directly.
///
/// An empty `windows` is "no window", so `false` — the "admits everything"
/// reading belongs to the caller that knows an empty list means unrestricted.
#[must_use]
pub fn version_in_any_window(version: &str, windows: &[(&str, Option<&str>)]) -> bool {
    let Some(have) = ParsedVersion::parse(version) else {
        return false;
    };
    windows
        .iter()
        .any(|&(from, until)| satisfies_range(&have.segments, from, until))
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
    select_package_version_for(available, requirements, prefer, TclVersion::V8_6)
}

/// Select a package provider using a pinned Tcl release's grammar.
#[must_use]
pub fn select_package_version_for<S: AsRef<str>>(
    available: &[S],
    requirements: &[&str],
    prefer: PackagePrefer,
    release: TclVersion,
) -> Option<usize> {
    let available: Vec<&[u8]> = available
        .iter()
        .map(|version| version.as_ref().as_bytes())
        .collect();
    let requirements: Vec<&[u8]> = requirements
        .iter()
        .map(|requirement| requirement.as_bytes())
        .collect();
    select_package_version_bytes_for(&available, &requirements, prefer, release)
}

/// Select a package provider from original byte spellings using the shared grammar.
#[must_use]
pub fn select_package_version_bytes_for<S: AsRef<[u8]>>(
    available: &[S],
    requirements: &[&[u8]],
    prefer: PackagePrefer,
    release: TclVersion,
) -> Option<usize> {
    use core::cmp::Ordering;
    if (!release.has_package_requirements() && requirements.len() > 1)
        || requirements
            .iter()
            .any(|requirement| validate_requirement_bytes_for(requirement, release).is_err())
    {
        return None;
    }
    let allow_plus_suffix = release.allows_package_version_suffix();
    let mut best: Option<(usize, SegmentBuf<'_>)> = None;
    let mut best_stable: Option<(usize, SegmentBuf<'_>)> = None;
    for (i, candidate) in available.iter().enumerate() {
        if !validate_version_bytes_for(candidate.as_ref(), release) {
            continue;
        }
        let Some(parsed) = ParsedVersion::parse_bytes_for(candidate.as_ref(), allow_plus_suffix)
        else {
            continue;
        };
        if !requirements.is_empty()
            && !requirements
                .iter()
                .any(|r| satisfies_internal(&parsed.segments, r, allow_plus_suffix))
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
    match (release.has_package_requirements(), prefer) {
        (true, PackagePrefer::Stable) => best_stable.or(best),
        (false, _) | (true, PackagePrefer::Latest) => best,
    }
    .map(|(i, _)| i)
}

/// Select an ifneeded provider that exactly matches a requested package
/// version using a pinned Tcl release's grammar.
#[must_use]
pub fn select_package_version_exact_for<S: AsRef<str>>(
    available: &[S],
    requested: &str,
    release: TclVersion,
) -> Option<usize> {
    let available: Vec<&[u8]> = available
        .iter()
        .map(|version| version.as_ref().as_bytes())
        .collect();
    select_package_version_exact_bytes_for(&available, requested.as_bytes(), release)
}

/// Select a package provider from original byte spellings using the shared grammar.
#[must_use]
pub fn select_package_version_exact_bytes_for<S: AsRef<[u8]>>(
    available: &[S],
    requested: &[u8],
    release: TclVersion,
) -> Option<usize> {
    if !validate_version_bytes_for(requested, release) {
        return None;
    }
    let allow_plus_suffix = release.allows_package_version_suffix();
    let requested = ParsedVersion::parse_bytes_for(requested, allow_plus_suffix)?;
    available.iter().enumerate().find_map(|(index, candidate)| {
        if !validate_version_bytes_for(candidate.as_ref(), release) {
            return None;
        }
        let candidate = ParsedVersion::parse_bytes_for(candidate.as_ref(), allow_plus_suffix)?;
        (compare_internal(&candidate.segments, &requested.segments).0 == core::cmp::Ordering::Equal)
            .then_some(index)
    })
}
