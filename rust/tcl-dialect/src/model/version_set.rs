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

//! The axis-typed version-set algebra of design doc
//! `docs/design/dialect-and-package-registry-redesign.md` §4.1 (review
//! finding B3), plus the per-item [`ItemHistory`].
//!
//! Two version types, deliberately: [`ItemHistory`] answers "when was
//! this one item introduced, deprecated, retired"; [`VersionSet`] answers
//! requirement/target set algebra. Tcl requirements are alternatives of
//! ranges with **exclusive maxima**, so requirements and targets are
//! normalised unions of half-open ranges, never a single interval, and
//! every set carries its [`VersionAxisId`] so a Tcl core release, a
//! package version, and a BIG-IP release can never be compared by
//! accident (invariant I2 — an axis mismatch on any binary operation is
//! a typed error, not a coercion).
//!
//! The requirement translation follows `tclPkg.c` exactly, including the
//! `a0` padding rule: a bare `5.0` requirement reads as `5.0a0-6a0`
//! (`tmp/tcl9.0.4/tests/package.test:1155-1164`), so `5.0a0` satisfies a
//! plain `5.0` requirement. The one comparator is the existing
//! [`compare_versions`] port — nothing here re-implements version
//! ordering — and the whole construction is differentially tested against
//! the pinned `package vsatisfies` corpus in
//! `tests/data/package_version_oracle.txt`.

use std::sync::Arc;

use crate::model::family::Family;
use crate::version::{compare_versions, version_is_stable, version_satisfies};

/// The interned identity of one version axis (§4.1): a family's core
/// release ladder or a named package's own version train.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VersionAxisId(AxisInner);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum AxisInner {
    Core(Family),
    Package(Arc<str>),
}

impl VersionAxisId {
    /// The core axis of `family` — the axis `package require Tcl` floors
    /// and core targets live on.
    #[must_use]
    pub const fn core(family: Family) -> Self {
        Self(AxisInner::Core(family))
    }

    /// The version axis of the package named `name` (`"Tk"`,
    /// `"struct::graph"`, …).
    #[must_use]
    pub fn package(name: &str) -> Self {
        Self(AxisInner::Package(Arc::from(name)))
    }

    /// The core family, when this is a core axis.
    #[must_use]
    pub fn core_family(&self) -> Option<Family> {
        match &self.0 {
            AxisInner::Core(family) => Some(*family),
            AxisInner::Package(_) => None,
        }
    }

    /// The package name, when this is a package axis.
    #[must_use]
    pub fn package_name(&self) -> Option<&str> {
        match &self.0 {
            AxisInner::Core(_) => None,
            AxisInner::Package(name) => Some(name),
        }
    }
}

impl std::fmt::Display for VersionAxisId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            AxisInner::Core(family) => write!(f, "core:{family}"),
            AxisInner::Package(name) => write!(f, "package:{name}"),
        }
    }
}

/// A concrete version string, validated against the `tclPkg.c` grammar
/// and ordered by the one ported comparator ([`compare_versions`]).
///
/// Equality is **comparator equality** — `"1.2"`, `"1.2.0"`, and
/// `"1.2.0.0"` are the same version (trailing zero components are not
/// significant) — which is why the type deliberately does not implement
/// `Hash`.
#[derive(Debug, Clone)]
pub struct Version {
    text: Arc<str>,
}

impl Version {
    /// Parse and validate a version string.
    ///
    /// # Errors
    /// [`VersionSetError::InvalidVersion`] when `text` is not a
    /// well-formed TIP 268 version (the strings real Tcl raises
    /// `expected version number` on).
    pub fn parse(text: &str) -> Result<Self, VersionSetError> {
        // Every well-formed version satisfies the unbounded `0-`
        // requirement and no malformed string does, so the shipping
        // `version_satisfies` doubles as the validator without exposing
        // the private parser.
        if version_satisfies(text, "0-") {
            Ok(Self {
                text: Arc::from(text),
            })
        } else {
            Err(VersionSetError::InvalidVersion(text.to_owned()))
        }
    }

    /// The version's original spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        compare_versions(&self.text, &other.text) == std::cmp::Ordering::Equal
    }
}

impl Eq for Version {}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        compare_versions(&self.text, &other.text)
    }
}

/// A typed error from version-set construction or algebra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionSetError {
    /// Two sets on different axes met in a binary operation (design
    /// invariant I2).
    AxisMismatch {
        /// The left operand's axis.
        left: VersionAxisId,
        /// The right operand's axis.
        right: VersionAxisId,
    },
    /// A string that is not a well-formed TIP 268 version.
    InvalidVersion(String),
    /// A requirement string real `package vsatisfies` would raise on
    /// (second dash, malformed bound, empty minimum).
    InvalidRequirement(String),
}

impl std::fmt::Display for VersionSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AxisMismatch { left, right } => {
                write!(f, "version axes differ: `{left}` vs `{right}`")
            }
            Self::InvalidVersion(text) => write!(f, "expected version number but got `{text}`"),
            Self::InvalidRequirement(text) => {
                write!(f, "expected version requirement but got `{text}`")
            }
        }
    }
}

impl std::error::Error for VersionSetError {}

/// One normalised range of a [`VersionSet`].
///
/// Almost every range is a half-open span `[min, max)` under the
/// comparator order — the shape every non-exact Tcl requirement
/// translates to once its bounds carry the `a0` pad. The degenerate
/// `V-V` requirement (`package require -exact`) admits exactly the
/// versions comparator-equal to `V`, which no half-open span with
/// spellable endpoints can express, so exact points are their own
/// variant — "exact points where the comparator requires them" (§4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HalfOpenRange {
    /// `[min, max)`: `min` inclusive, `max` exclusive; `None` = unbounded
    /// above.
    Span {
        /// The inclusive lower bound.
        min: Version,
        /// The exclusive upper bound; `None` = unbounded.
        max: Option<Version>,
    },
    /// Exactly the versions comparator-equal to the point (`-exact`).
    Exact(Version),
}

impl HalfOpenRange {
    /// Whether `version` is in this range.
    #[must_use]
    pub fn contains(&self, version: &Version) -> bool {
        match self {
            Self::Span { min, max } => {
                version >= min && max.as_ref().is_none_or(|max| version < max)
            }
            Self::Exact(point) => version == point,
        }
    }

    /// Whether the range admits no version at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Span { min, max } => max.as_ref().is_some_and(|max| min >= max),
            Self::Exact(_) => false,
        }
    }

    /// The inclusive lower bound (a span's `min`, an exact point
    /// itself).
    fn lower(&self) -> &Version {
        match self {
            Self::Span { min, .. } => min,
            Self::Exact(point) => point,
        }
    }

    /// Sort rank at an equal lower bound: spans before exacts, so a
    /// sweep absorbs `Exact(e)` into a span starting at `e`.
    fn tier(&self) -> u8 {
        match self {
            Self::Span { .. } => 0,
            Self::Exact(_) => 1,
        }
    }
}

/// A normalised union of half-open ranges on one named axis (§4.1):
/// sorted, disjoint, merged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionSet {
    axis: VersionAxisId,
    ranges: Arc<[HalfOpenRange]>,
}

impl VersionSet {
    /// The empty set on `axis`.
    #[must_use]
    pub fn empty(axis: VersionAxisId) -> Self {
        Self {
            axis,
            ranges: Arc::from([]),
        }
    }

    /// A set from arbitrary ranges, normalised (empty ranges dropped;
    /// overlapping, adjacent, and absorbed ranges merged; sorted).
    #[must_use]
    pub fn from_ranges(axis: VersionAxisId, ranges: Vec<HalfOpenRange>) -> Self {
        Self {
            axis,
            ranges: normalise(ranges).into(),
        }
    }

    /// A set from Tcl requirement syntax: each requirement is `min`
    /// (bounded at the next major), `min-` (unbounded above), `min-max`
    /// (max exclusive), or the degenerate `v-v` exact form; multiple
    /// requirements union, exactly as `package vsatisfies`'s OR does.
    ///
    /// Bounds carry the `a0` pad of `tclPkg.c` (`5.0` ⇒ `5.0a0-6a0`), so
    /// an alpha of the bound's own version satisfies it.
    ///
    /// # Errors
    /// [`VersionSetError::InvalidRequirement`] /
    /// [`VersionSetError::InvalidVersion`] on the strings real Tcl raises
    /// on. An unsatisfiable-but-well-formed requirement (`2.0-1.0`) is
    /// not an error — it is the empty set, as in Tcl.
    pub fn from_requirements<S: AsRef<str>>(
        axis: VersionAxisId,
        requirements: &[S],
    ) -> Result<Self, VersionSetError> {
        let mut ranges = Vec::with_capacity(requirements.len());
        for requirement in requirements {
            ranges.push(range_of_requirement(requirement.as_ref())?);
        }
        Ok(Self::from_ranges(axis, ranges))
    }

    /// The set's axis.
    #[must_use]
    pub fn axis(&self) -> &VersionAxisId {
        &self.axis
    }

    /// The normalised ranges, sorted by lower bound.
    #[must_use]
    pub fn ranges(&self) -> &[HalfOpenRange] {
        &self.ranges
    }

    /// Whether the set admits no version.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    /// Whether `version` is in the set.
    #[must_use]
    pub fn contains(&self, version: &Version) -> bool {
        self.ranges.iter().any(|range| range.contains(version))
    }

    /// The intersection of two sets on the same axis.
    ///
    /// # Errors
    /// [`VersionSetError::AxisMismatch`] when the axes differ (I2).
    pub fn intersect(&self, other: &Self) -> Result<Self, VersionSetError> {
        self.check_axis(other)?;
        let mut ranges = Vec::new();
        for a in self.ranges.iter() {
            for b in other.ranges.iter() {
                if let Some(range) = intersect_ranges(a, b) {
                    ranges.push(range);
                }
            }
        }
        Ok(Self::from_ranges(self.axis.clone(), ranges))
    }

    /// The union of two sets on the same axis.
    ///
    /// # Errors
    /// [`VersionSetError::AxisMismatch`] when the axes differ (I2).
    pub fn union(&self, other: &Self) -> Result<Self, VersionSetError> {
        self.check_axis(other)?;
        let mut ranges = self.ranges.to_vec();
        ranges.extend(other.ranges.iter().cloned());
        Ok(Self::from_ranges(self.axis.clone(), ranges))
    }

    /// Whether every version in `self` is in `other`.
    ///
    /// # Errors
    /// [`VersionSetError::AxisMismatch`] when the axes differ (I2).
    pub fn subset(&self, other: &Self) -> Result<bool, VersionSetError> {
        // Normal forms are canonical, so A ⊆ B ⇔ A ∩ B = A.
        Ok(self.intersect(other)? == *self)
    }

    fn check_axis(&self, other: &Self) -> Result<(), VersionSetError> {
        if self.axis == other.axis {
            Ok(())
        } else {
            Err(VersionSetError::AxisMismatch {
                left: self.axis.clone(),
                right: other.axis.clone(),
            })
        }
    }
}

/// Compare two exclusive upper bounds, `None` = +∞.
fn cmp_max(a: Option<&Version>, b: Option<&Version>) -> std::cmp::Ordering {
    match (a, b) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (Some(a), Some(b)) => a.cmp(b),
    }
}

/// The intersection of two single ranges, if non-empty.
fn intersect_ranges(a: &HalfOpenRange, b: &HalfOpenRange) -> Option<HalfOpenRange> {
    match (a, b) {
        (HalfOpenRange::Exact(p), other) | (other, HalfOpenRange::Exact(p)) => {
            other.contains(p).then(|| HalfOpenRange::Exact(p.clone()))
        }
        (
            HalfOpenRange::Span {
                min: min_a,
                max: max_a,
            },
            HalfOpenRange::Span {
                min: min_b,
                max: max_b,
            },
        ) => {
            let min = std::cmp::max(min_a, min_b).clone();
            let max = match cmp_max(max_a.as_ref(), max_b.as_ref()) {
                std::cmp::Ordering::Greater => max_b.clone(),
                _ => max_a.clone(),
            };
            let range = HalfOpenRange::Span { min, max };
            (!range.is_empty()).then_some(range)
        }
    }
}

/// One step of the normalising sweep: what to do with `next`, given the
/// last already-merged range.
enum Sweep {
    /// `next` starts a new disjoint range.
    Push,
    /// `next` is entirely covered — drop it.
    Drop,
    /// `next` extends the current span's exclusive max to this bound.
    ExtendMax(Option<Version>),
    /// `next` (a span starting at the current exact point) covers the
    /// point — replace it.
    Replace,
}

fn sweep_action(current: &HalfOpenRange, next: &HalfOpenRange) -> Sweep {
    match (current, next) {
        (
            HalfOpenRange::Span { max, .. },
            HalfOpenRange::Span {
                min: min_b,
                max: max_b,
            },
        ) => {
            // Merge when overlapping or exactly adjacent
            // ([a,b) ∪ [b,c) = [a,c)).
            if max.as_ref().is_none_or(|max| min_b <= max) {
                if cmp_max(max_b.as_ref(), max.as_ref()) == std::cmp::Ordering::Greater {
                    Sweep::ExtendMax(max_b.clone())
                } else {
                    Sweep::Drop
                }
            } else {
                Sweep::Push
            }
        }
        (HalfOpenRange::Span { .. }, HalfOpenRange::Exact(point)) => {
            // Absorb a point the span already covers; a point AT the
            // exclusive max ([a,b) then {b}) stays its own range.
            if current.contains(point) {
                Sweep::Drop
            } else {
                Sweep::Push
            }
        }
        (HalfOpenRange::Exact(point), HalfOpenRange::Span { min, .. }) => {
            // A span starting at the point covers it.
            if point == min {
                Sweep::Replace
            } else {
                Sweep::Push
            }
        }
        (HalfOpenRange::Exact(a), HalfOpenRange::Exact(b)) => {
            if a == b {
                Sweep::Drop
            } else {
                Sweep::Push
            }
        }
    }
}

/// Normalise: drop empties, sort by lower bound (spans before exact
/// points at an equal bound), then sweep-merge overlapping and adjacent
/// spans and absorb covered exact points. Idempotent.
fn normalise(mut ranges: Vec<HalfOpenRange>) -> Vec<HalfOpenRange> {
    ranges.retain(|range| !range.is_empty());
    ranges.sort_by(|a, b| {
        a.lower()
            .cmp(b.lower())
            .then_with(|| a.tier().cmp(&b.tier()))
    });
    let mut merged: Vec<HalfOpenRange> = Vec::with_capacity(ranges.len());
    for next in ranges {
        let Some(current) = merged.last_mut() else {
            merged.push(next);
            continue;
        };
        match sweep_action(current, &next) {
            Sweep::Push => merged.push(next),
            Sweep::Drop => {}
            Sweep::ExtendMax(bound) => {
                if let HalfOpenRange::Span { max, .. } = current {
                    *max = bound;
                }
            }
            Sweep::Replace => *current = next,
        }
    }
    merged
}

/// The inclusive lower bound of a requirement's `min`, carrying the `a0`
/// pad: a stable bound gains a literal `a0` component (`8.5` → `8.5a0`,
/// exactly the spellable form of `tclPkg.c`'s pushed alpha segment); an
/// unstable bound (`1.3b1`) cannot spell a second marker, but no
/// spellable version lies strictly between `1.3b1a0` and `1.3b1`, so the
/// bound itself, inclusive, is the same set.
fn padded_bound(bound: &str) -> Result<Version, VersionSetError> {
    let version = Version::parse(bound)?;
    if version_is_stable(bound) {
        Ok(Version {
            text: Arc::from(format!("{bound}a0")),
        })
    } else {
        Ok(version)
    }
}

/// The exclusive upper bound of a bare `min` requirement: the next major
/// (`tclPkg.c` bounds a dashless requirement at the major after its
/// min's), as a digit string so arbitrary-width majors stay exact.
fn next_major(min: &str) -> String {
    let digits: &str = &min[..min.find(|c: char| !c.is_ascii_digit()).unwrap_or(min.len())];
    let mut digits: Vec<u8> = digits.trim_start_matches('0').bytes().collect();
    let mut carry = true;
    for digit in digits.iter_mut().rev() {
        if *digit == b'9' {
            *digit = b'0';
        } else {
            *digit += 1;
            carry = false;
            break;
        }
    }
    if carry {
        digits.insert(0, b'1');
    }
    String::from_utf8(digits).expect("decimal digits are UTF-8")
}

/// One requirement string as a range, per `tclPkg.c`'s
/// `CheckRequirement` / `RequirementSatisfied`.
fn range_of_requirement(requirement: &str) -> Result<HalfOpenRange, VersionSetError> {
    let invalid = || VersionSetError::InvalidRequirement(requirement.to_owned());
    let Some((lo, hi)) = requirement.split_once('-') else {
        // Bare `min`: `[min·a0, (major+1)·a0)`.
        let min = padded_bound(requirement)?;
        let max = Version {
            text: Arc::from(format!("{}a0", next_major(requirement))),
        };
        return Ok(HalfOpenRange::Span {
            min,
            max: Some(max),
        });
    };
    // `CheckRequirement`: at most one dash.
    if hi.contains('-') || lo.is_empty() {
        return Err(invalid());
    }
    if hi.is_empty() {
        // `min-`: unbounded above.
        return Ok(HalfOpenRange::Span {
            min: padded_bound(lo)?,
            max: None,
        });
    }
    let min = Version::parse(lo)?;
    let max = Version::parse(hi)?;
    if min == max {
        // The degenerate exact form is compared unpadded.
        return Ok(HalfOpenRange::Exact(min));
    }
    Ok(HalfOpenRange::Span {
        min: padded_bound(lo)?,
        max: Some(padded_bound(hi)?),
    })
}

/// One item's own story on one axis (§4.1's `ItemHistory`): the first
/// release where it exists, where it warns, and where it no longer
/// exists.
///
/// The semantics replicate `tcl-registry`'s `Lifecycle` exactly (that
/// crate sits above this one, so the rules are restated rather than
/// imported, with its edge tests carried over): availability is
/// `introduced <= target < retired`, retirement is **exclusive** and
/// outranks deprecation, deprecation applies only while still available,
/// and an unknown target is permissive. Absent releases mean the history
/// has not reached that state; there is deliberately no generic "maximum
/// version" — an upper bound exists only as retirement metadata.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ItemHistory {
    /// First release where the item exists. `None` = present in every
    /// release of its axis.
    pub introduced: Option<Version>,
    /// First release where the item is still available but deprecated.
    pub deprecated: Option<Version>,
    /// First release where the item no longer exists — **exclusive**.
    pub retired: Option<Version>,
}

/// Where an item sits relative to a target release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ItemState {
    /// The target predates [`ItemHistory::introduced`].
    NotIntroduced,
    /// Available, with no deprecation declared or not yet reached.
    Available,
    /// Available but deprecated at the target.
    Deprecated,
    /// The target is at or past [`ItemHistory::retired`].
    Retired,
}

impl ItemState {
    /// Whether the item exists at all at the target.
    #[must_use]
    pub const fn is_available(self) -> bool {
        matches!(self, Self::Available | Self::Deprecated)
    }
}

/// An impossible [`ItemHistory`] ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemHistoryError {
    /// `deprecated` predates `introduced`.
    DeprecatedBeforeIntroduced,
    /// `retired` predates `introduced`.
    RetiredBeforeIntroduced,
    /// `retired` predates `deprecated`.
    RetiredBeforeDeprecated,
}

impl std::fmt::Display for ItemHistoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::DeprecatedBeforeIntroduced => {
                "deprecated release predates the introducing release"
            }
            Self::RetiredBeforeIntroduced => "retired release predates the introducing release",
            Self::RetiredBeforeDeprecated => "retired release predates the deprecating release",
        })
    }
}

impl std::error::Error for ItemHistoryError {}

impl ItemHistory {
    /// The item's state at `target`. `None` (unknown target release) is
    /// permissive and yields [`ItemState::Available`].
    #[must_use]
    pub fn state_at(&self, target: Option<&Version>) -> ItemState {
        let Some(target) = target else {
            return ItemState::Available;
        };
        if self
            .retired
            .as_ref()
            .is_some_and(|retired| target >= retired)
        {
            return ItemState::Retired;
        }
        if self
            .introduced
            .as_ref()
            .is_some_and(|introduced| target < introduced)
        {
            return ItemState::NotIntroduced;
        }
        if self
            .deprecated
            .as_ref()
            .is_some_and(|deprecated| target >= deprecated)
        {
            return ItemState::Deprecated;
        }
        ItemState::Available
    }

    /// Whether the item exists at `target` —
    /// `introduced <= target < retired`.
    #[must_use]
    pub fn available_at(&self, target: Option<&Version>) -> bool {
        self.state_at(target).is_available()
    }

    /// Whether the item is deprecated *and still available* at `target`.
    #[must_use]
    pub fn deprecated_at(&self, target: Option<&Version>) -> bool {
        self.state_at(target) == ItemState::Deprecated
    }

    /// Whether the item is retired at `target`.
    #[must_use]
    pub fn retired_at(&self, target: Option<&Version>) -> bool {
        self.state_at(target) == ItemState::Retired
    }

    /// Reject impossible orderings.
    ///
    /// # Errors
    /// The first ordering violation found; `deprecated == retired` in the
    /// same release is legal.
    pub fn validate(&self) -> Result<(), ItemHistoryError> {
        if let (Some(introduced), Some(deprecated)) = (&self.introduced, &self.deprecated)
            && deprecated < introduced
        {
            return Err(ItemHistoryError::DeprecatedBeforeIntroduced);
        }
        if let (Some(introduced), Some(retired)) = (&self.introduced, &self.retired)
            && retired < introduced
        {
            return Err(ItemHistoryError::RetiredBeforeIntroduced);
        }
        if let (Some(deprecated), Some(retired)) = (&self.deprecated, &self.retired)
            && retired < deprecated
        {
            return Err(ItemHistoryError::RetiredBeforeDeprecated);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).expect("test version")
    }

    fn axis() -> VersionAxisId {
        VersionAxisId::package("demo")
    }

    fn set(requirements: &[&str]) -> VersionSet {
        VersionSet::from_requirements(axis(), requirements).expect("test requirements")
    }

    #[test]
    fn version_parse_accepts_the_tip_268_grammar_only() {
        for good in ["0", "1.2", "8.5.19", "1.2a1", "1.3b1", "0005", "1.2a0.1"] {
            assert!(Version::parse(good).is_ok(), "{good}");
        }
        for bad in ["", "1.", ".1", "1..2", "1a", "1a1b2", " 1.2", "x", "1.2-"] {
            assert_eq!(
                Version::parse(bad),
                Err(VersionSetError::InvalidVersion(bad.to_owned())),
                "{bad}"
            );
        }
    }

    #[test]
    fn version_equality_is_comparator_equality() {
        assert_eq!(v("1.2"), v("1.2.0"));
        assert_eq!(v("0005"), v("5"));
        assert!(v("1.2a1") < v("1.2"));
        assert!(v("1.10") > v("1.2"));
    }

    #[test]
    fn bare_requirement_is_bounded_at_the_next_major() {
        let s = set(&["8.5"]);
        assert!(s.contains(&v("8.5")));
        assert!(s.contains(&v("8.6")));
        assert!(s.contains(&v("8.5.2")));
        assert!(!s.contains(&v("9")));
        assert!(!s.contains(&v("9.0a0")));
        assert!(!s.contains(&v("8.4")));
        // The a0 pad: an alpha of the bound's own version satisfies it
        // (`tmp/tcl9.0.4/tests/package.test:1155-1164`).
        assert!(s.contains(&v("8.5a0")));
        assert!(set(&["5.0"]).contains(&v("5.0a0")));
    }

    #[test]
    fn open_and_closed_ranges_follow_vsatisfies() {
        let open = set(&["8.5-"]);
        assert!(open.contains(&v("9.0")));
        assert!(open.contains(&v("8.5a0")));
        assert!(!open.contains(&v("8.4.19")));

        let closed = set(&["8.5-9.0"]);
        assert!(closed.contains(&v("8.6")));
        assert!(closed.contains(&v("8.5a1")));
        assert!(!closed.contains(&v("9.0")), "the max is exclusive");
        assert!(!closed.contains(&v("9.0a0")), "the max is padded too");
    }

    #[test]
    fn exact_requirements_admit_only_the_point() {
        let exact = set(&["8.6-8.6"]);
        assert!(exact.contains(&v("8.6")));
        assert!(exact.contains(&v("8.6.0")), "trailing zeros are equal");
        assert!(!exact.contains(&v("8.6.14")));
        assert!(!exact.contains(&v("8.6a1")), "no alpha pad on exact");
    }

    #[test]
    fn multiple_requirements_union() {
        // The tcllib head-guard shape: `vsatisfies $v 8.5 9`.
        let s = set(&["8.5", "9"]);
        assert!(s.contains(&v("8.6")));
        assert!(s.contains(&v("9.0.4")));
        assert!(!s.contains(&v("8.4")));
        assert!(!s.contains(&v("10.0")));
    }

    #[test]
    fn malformed_requirements_are_typed_errors() {
        for bad in ["1.2-1.3-1.4", "-1.2", "", "a-b", "1.2-x", " 8.6", "8.6."] {
            assert!(
                VersionSet::from_requirements(axis(), &[bad]).is_err(),
                "{bad}"
            );
        }
        // Well-formed but unsatisfiable is the empty set, not an error.
        let empty = set(&["2.0-1.0"]);
        assert!(empty.is_empty());
        assert!(!empty.contains(&v("1.5")));
    }

    #[test]
    fn normalisation_merges_and_sorts() {
        // Overlap.
        let s = set(&["8.5-9.0", "8.6-9.1"]);
        assert_eq!(s.ranges().len(), 1);
        assert!(s.contains(&v("9.0")));
        // Exact adjacency: [a,b) ∪ [b,c) = [a,c).
        let adjacent = set(&["8.5-8.6", "8.6-8.7"]);
        assert_eq!(adjacent.ranges().len(), 1);
        assert!(adjacent.contains(&v("8.6")));
        // A covered exact point is absorbed; one at the exclusive max is
        // not.
        let absorbed = set(&["8.5-9.0", "8.6-8.6"]);
        assert_eq!(absorbed.ranges().len(), 1);
        let boundary = set(&["8.5-9.0", "9.0-9.0"]);
        assert_eq!(boundary.ranges().len(), 2);
        assert!(boundary.contains(&v("9.0")));
        assert!(!boundary.contains(&v("9.0.1")));
        // Disjoint ranges sort by lower bound.
        let disjoint = set(&["9", "1.2"]);
        assert_eq!(disjoint.ranges().len(), 2);
        assert!(disjoint.ranges()[0].contains(&v("1.5")));
        assert!(disjoint.ranges()[1].contains(&v("9.5")));
    }

    #[test]
    fn normalisation_is_idempotent() {
        for requirements in [
            &["8.5", "8.6-9.1", "9.0-9.0", "1.2a1-1.3", "8.5-"][..],
            &["2.0-1.0"],
            &["1.2-1.2", "1.2-1.3", "1.1-1.2"],
        ] {
            let s = set(requirements);
            let renormalised = VersionSet::from_ranges(axis(), s.ranges().to_vec());
            assert_eq!(s, renormalised, "{requirements:?}");
        }
    }

    #[test]
    fn set_algebra_is_coherent() {
        let a = set(&["8.5-9.0"]);
        let b = set(&["8.6", "9.0-9.1"]);
        let both = a.intersect(&b).expect("same axis");
        let either = a.union(&b).expect("same axis");
        for probe in ["8.4", "8.5", "8.6", "8.6.4", "8.7", "9.0", "9.0.5", "9.1"] {
            let probe = v(probe);
            assert_eq!(
                both.contains(&probe),
                a.contains(&probe) && b.contains(&probe),
                "{probe}"
            );
            assert_eq!(
                either.contains(&probe),
                a.contains(&probe) || b.contains(&probe),
                "{probe}"
            );
        }
        assert!(both.subset(&a).expect("same axis"));
        assert!(both.subset(&b).expect("same axis"));
        assert!(a.subset(&either).expect("same axis"));
        assert!(!a.subset(&b).expect("same axis"));
        assert!(a.subset(&a).expect("same axis"));
        assert!(VersionSet::empty(axis()).subset(&a).expect("same axis"));
    }

    #[test]
    fn axis_mismatch_is_a_typed_error_on_every_binary_op() {
        let core = VersionSet::from_requirements(VersionAxisId::core(Family::Tcl), &["8.5"])
            .expect("core requirement");
        let package = set(&["8.5"]);
        for result in [
            core.intersect(&package).err(),
            core.union(&package).err(),
            core.subset(&package).err(),
        ] {
            assert!(
                matches!(result, Some(VersionSetError::AxisMismatch { .. })),
                "{result:?}"
            );
        }
        // Distinct package axes mismatch too.
        let other = VersionSet::from_requirements(VersionAxisId::package("other"), &["8.5"])
            .expect("package requirement");
        assert!(package.intersect(&other).is_err());
        // Same axis by content, not pointer.
        assert!(package.union(&set(&["9"])).is_ok());
    }

    #[test]
    fn next_major_increments_digit_strings_exactly() {
        assert_eq!(next_major("8.5"), "9");
        assert_eq!(next_major("9.0"), "10");
        assert_eq!(next_major("0.84"), "1");
        assert_eq!(next_major("99.1"), "100");
        assert_eq!(next_major("1.3b1"), "2");
        assert_eq!(
            next_major("9999999999999999999999999999999999999999"),
            "10000000000000000000000000000000000000000"
        );
    }

    // ItemHistory: the Lifecycle edge rules, carried over from
    // rust/tcl-registry/src/lifecycle.rs's own tests.

    fn history(
        introduced: Option<&str>,
        deprecated: Option<&str>,
        retired: Option<&str>,
    ) -> ItemHistory {
        ItemHistory {
            introduced: introduced.map(v),
            deprecated: deprecated.map(v),
            retired: retired.map(v),
        }
    }

    #[test]
    fn unspecified_history_is_always_available() {
        let life = ItemHistory::default();
        assert_eq!(life.state_at(Some(&v("8.4"))), ItemState::Available);
        assert_eq!(life.state_at(None), ItemState::Available);
        assert!(!life.deprecated_at(Some(&v("9.0"))));
    }

    #[test]
    fn introduction_gates_the_lower_bound() {
        let life = history(Some("8.5"), None, None);
        assert_eq!(life.state_at(Some(&v("8.4"))), ItemState::NotIntroduced);
        assert!(!life.available_at(Some(&v("8.4"))));
        assert_eq!(life.state_at(Some(&v("8.5"))), ItemState::Available);
        assert_eq!(life.state_at(Some(&v("9.0"))), ItemState::Available);
        assert!(life.available_at(None), "unknown target stays permissive");
    }

    #[test]
    fn retirement_is_exclusive_and_outranks_deprecation() {
        // The classic-XML shape: present from 9.0.3, gone from 10.0.0.
        let classic = history(Some("9.0.3"), Some("10.0.0"), Some("10.0.0"));
        assert_eq!(
            classic.state_at(Some(&v("9.0.2"))),
            ItemState::NotIntroduced
        );
        assert_eq!(classic.state_at(Some(&v("9.0.3"))), ItemState::Available);
        assert_eq!(classic.state_at(Some(&v("9.4.8"))), ItemState::Available);
        assert_eq!(classic.state_at(Some(&v("10.0.0"))), ItemState::Retired);
        assert_eq!(classic.state_at(Some(&v("11.0.0"))), ItemState::Retired);
        assert!(!classic.available_at(Some(&v("10.0.0"))));
        assert!(!classic.deprecated_at(Some(&v("10.0.0"))));
        assert!(classic.retired_at(Some(&v("10.0.0"))));
    }

    #[test]
    fn deprecation_only_applies_while_available() {
        let auth = history(Some("9.0.0"), Some("9.4.0"), None);
        assert_eq!(auth.state_at(Some(&v("9.3.0"))), ItemState::Available);
        assert_eq!(auth.state_at(Some(&v("9.4.0"))), ItemState::Deprecated);
        assert_eq!(auth.state_at(Some(&v("17.0.0"))), ItemState::Deprecated);
        assert!(auth.available_at(Some(&v("17.0.0"))));
    }

    #[test]
    fn history_validation_rejects_impossible_orderings() {
        assert_eq!(
            history(Some("9.0"), Some("8.6"), None).validate(),
            Err(ItemHistoryError::DeprecatedBeforeIntroduced)
        );
        assert_eq!(
            history(Some("9.0"), None, Some("8.6")).validate(),
            Err(ItemHistoryError::RetiredBeforeIntroduced)
        );
        assert_eq!(
            history(None, Some("9.0"), Some("8.6")).validate(),
            Err(ItemHistoryError::RetiredBeforeDeprecated)
        );
        // Deprecated == retired in the same release is legal.
        assert_eq!(
            history(Some("9.0.3"), Some("10.0.0"), Some("10.0.0")).validate(),
            Ok(())
        );
        assert_eq!(ItemHistory::default().validate(), Ok(()));
    }
}
