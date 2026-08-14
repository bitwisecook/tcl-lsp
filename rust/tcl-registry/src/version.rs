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

//! Tcl package version arithmetic — comparison and `package vsatisfies`.
//!
//! Tcl versions are dotted sequences of non-negative integers (`8`, `8.6`,
//! `8.6.16`, `2.0`).  Comparison is segment-wise with missing trailing
//! segments treated as zero, so `8.6` and `8.6.0` are equal.
//!
//! A *requirement* (as used by `package require` and `package vsatisfies`)
//! has one of three forms:
//!
//! * `min` — shorthand for `min-<M+1>` where `M` is `min`'s first (major)
//!   segment, i.e. the half-open range `[min, M+1)`. `8.6` admits `8.6`…`8.9`
//!   but not `9.0`; `2.1` is shorthand for `2.1-3`.
//! * `min-` — the unbounded range `[min, ∞)`.
//! * `min-max` — the half-open range `[min, max)` (`max` exclusive).
//!
//! This mirrors the Tcl `package` command's requirement semantics.  It is a
//! distinct scheme from the semver-flavoured [`tcl_pkg::Version`] used by the
//! package manager; the registry needs Tcl-native dotted-version gating for
//! command/option availability, so it is kept self-contained here.

use std::cmp::Ordering;

/// Iterator over a dotted version's numeric segments.  Non-numeric segments
/// terminate parsing (defensive; real Tcl versions are purely numeric).
fn segments(version: &str) -> impl Iterator<Item = u64> + '_ {
    version.split('.').map_while(|seg| {
        if seg.is_empty() {
            None
        } else {
            seg.parse::<u64>().ok()
        }
    })
}

/// Segment-wise comparison of two dotted versions, zero-padding the shorter.
///
/// **Prerelease-blind, by design and by domain.** [`segments`] stops at the
/// first non-numeric segment, so `1.0a1`, `1.0b1`, and `1.0` all compare
/// **equal**. That is correct for what this comparator exists to answer —
/// lifecycle floors and `package require` requirement bounds, whose release
/// strings are plain dotted versions ([`crate::lifecycle::Lifecycle`]) — and
/// it keeps a malformed spec from ordering unpredictably.
///
/// It is **not** correct for ordering or de-duplicating *release labels* a
/// package actually shipped, where a prerelease is a distinct release. Use
/// `tcl_dialect::compare_versions` — the oracle-pinned
/// `package vcompare` port — for that.
#[must_use]
pub fn compare(a: &str, b: &str) -> Ordering {
    let mut ai = segments(a);
    let mut bi = segments(b);
    loop {
        match (ai.next(), bi.next()) {
            (None, None) => return Ordering::Equal,
            (av, bv) => {
                let ord = av.unwrap_or(0).cmp(&bv.unwrap_or(0));
                if ord != Ordering::Equal {
                    return ord;
                }
            }
        }
    }
}

/// True when *available* version is >= *minimum* version.
#[must_use]
pub fn meets_min(available: &str, minimum: &str) -> bool {
    compare(available, minimum) != Ordering::Less
}

/// First (major) segment of a version, or 0.
fn first_segment(version: &str) -> u64 {
    segments(version).next().unwrap_or(0)
}

/// True when *version* satisfies a single *requirement* string.
#[must_use]
pub fn satisfies_one(version: &str, requirement: &str) -> bool {
    let req = requirement.trim();
    if let Some(min) = req.strip_suffix('-') {
        return compare(version, min) != Ordering::Less;
    }
    if let Some((lo, hi)) = req.split_once('-') {
        // Tcl's `RequirementSatisfied` special-cases an equal min/max: when the
        // two bounds compare equal the requirement is an *exact pin* rather than
        // the (empty) half-open range `[min, max)`.  `package vsatisfies 8.4
        // 8.4-8.4` is therefore true, not false.
        if compare(lo, hi) == Ordering::Equal {
            return compare(version, lo) == Ordering::Equal;
        }
        return compare(version, lo) != Ordering::Less && compare(version, hi) == Ordering::Less;
    }
    // Bare `min` == `[min, first(min)+1)`.
    if compare(version, req) == Ordering::Less {
        return false;
    }
    first_segment(version) <= first_segment(req)
}

/// Mirror `package vsatisfies`: true if *version* meets **any** requirement.
///
/// With no requirements the result is `true` (an unconstrained
/// `package require` accepts any version).
#[must_use]
pub fn vsatisfies(version: &str, requirements: &[&str]) -> bool {
    requirements.is_empty() || requirements.iter().any(|r| satisfies_one(version, r))
}

/// Return the minimum version a *requirement* admits.
///
/// `8.6` -> `8.6`, `8.6-` -> `8.6`, `8.6-9.0` -> `8.6`.  This is the
/// guaranteed-available floor of `package require pkg <requirement>`: the
/// runtime package is at least this version.
#[must_use]
pub fn requirement_lower_bound(requirement: &str) -> &str {
    let req = requirement.trim();
    if let Some(min) = req.strip_suffix('-') {
        return min;
    }
    if let Some((lo, _hi)) = req.split_once('-') {
        return lo;
    }
    req
}

/// Return the **exclusive** upper bound a *requirement* admits, when it
/// states one.
///
/// `8.6-9.0` -> `Some("9.0")`; `8.6-` -> `None` (unbounded above); `8.6` ->
/// `None`. The bare form's implicit `[min, major+1)` ceiling is deliberately
/// *not* reported: it is Tcl's shorthand rather than an author-stated
/// intention, and treating it as a declared ceiling would make every
/// unadorned `package require` claim a range it never meant.
///
/// The companion of [`requirement_lower_bound`]: together they describe the
/// whole window a `package require pkg <requirement>` may resolve to, which
/// is what a straddle check (does the window cross a retirement?) needs.
#[must_use]
pub fn requirement_upper_bound(requirement: &str) -> Option<&str> {
    let req = requirement.trim();
    if req.ends_with('-') {
        return None;
    }
    let (_lo, hi) = req.split_once('-')?;
    (!hi.is_empty()).then_some(hi)
}

/// Whether `version` falls inside the closed range `[min, max]` (either
/// bound absent = unbounded on that side). The versioned-library axis's
/// range test: `min` is the introducing release (or the axis baseline),
/// `max` the removing one (open today — nothing modelled is removed).
#[must_use]
pub fn within_range(version: &str, min: Option<&str>, max: Option<&str>) -> bool {
    if let Some(min) = min
        && !meets_min(version, min)
    {
        return false;
    }
    if let Some(max) = max
        && compare(version, max).is_gt()
    {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering::{Equal, Greater, Less};

    #[test]
    fn compare_pads_and_orders() {
        assert_eq!(compare("8.6", "8.6.0"), Equal);
        assert_eq!(compare("8.6", "8.7"), Less);
        assert_eq!(compare("9.0", "8.9"), Greater);
        assert_eq!(compare("2.0", "1.11.4"), Greater);
        assert_eq!(compare("8.6.16", "8.6.2"), Greater);
    }

    #[test]
    fn bare_requirement_is_major_bounded() {
        assert!(satisfies_one("8.6", "8.6"));
        assert!(satisfies_one("8.6.1", "8.6"));
        assert!(satisfies_one("8.9", "8.6"));
        assert!(!satisfies_one("9.0", "8.6")); // next major excluded
        assert!(!satisfies_one("8.5", "8.6"));
        // `2.1` shorthand for `[2.1, 3)`.
        assert!(satisfies_one("2.5", "2.1"));
        assert!(!satisfies_one("3.0", "2.1"));
    }

    #[test]
    fn unbounded_and_ranged_requirements() {
        assert!(satisfies_one("9.0", "8.6-"));
        assert!(satisfies_one("8.6", "8.6-"));
        assert!(satisfies_one("8.6", "8.4-9.0"));
        assert!(!satisfies_one("9.0", "8.4-9.0")); // max exclusive
    }

    #[test]
    fn equal_min_max_is_exact_pin() {
        // Tcl `RequirementSatisfied`: min == max degenerates to an exact pin,
        // not the empty half-open range. `package vsatisfies 8.4 8.4-8.4` -> 1.
        assert!(satisfies_one("8.4", "8.4-8.4"));
        assert!(satisfies_one("8.4.0", "8.4-8.4")); // zero-padding still equal
        assert!(!satisfies_one("8.5", "8.4-8.4"));
        assert!(!satisfies_one("8.3", "8.4-8.4"));
        // A genuine (non-degenerate) range keeps the exclusive upper bound.
        assert!(!satisfies_one("8.5", "8.4-8.5"));
        assert!(satisfies_one("8.4", "8.4-8.5"));
    }

    #[test]
    fn vsatisfies_is_any_of() {
        assert!(vsatisfies("9.0", &["8.6", "9.0"]));
        assert!(vsatisfies("8.7", &[])); // no requirement -> permissive
        assert!(!vsatisfies("8.5", &["8.6", "9.0"]));
    }

    #[test]
    fn lower_bound_and_meets_min() {
        assert_eq!(requirement_lower_bound("8.7"), "8.7");
        assert_eq!(requirement_lower_bound("8.6-9.0"), "8.6");
        assert_eq!(requirement_lower_bound("8.5-"), "8.5");
        assert!(meets_min("8.7", "8.7"));
        assert!(meets_min("9.0", "8.7"));
        assert!(!meets_min("8.6", "8.7"));
    }

    #[test]
    fn upper_bound_is_stated_only_by_a_real_range() {
        // Only `a-b` states a ceiling, and it is the exclusive `b`.
        assert_eq!(requirement_upper_bound("8.6-9.0"), Some("9.0"));
        assert_eq!(requirement_upper_bound("1.0-3.0"), Some("3.0"));
        // The degenerate exact pin still reports its (equal) upper end, so a
        // straddle check sees the same window `satisfies_one` accepts.
        assert_eq!(requirement_upper_bound("8.4-8.4"), Some("8.4"));
        // `a-` is unbounded above; a bare `a` states no ceiling of its own
        // (its `[a, major+1)` window is Tcl shorthand, not an author claim).
        assert_eq!(requirement_upper_bound("8.5-"), None);
        assert_eq!(requirement_upper_bound("8.7"), None);
        // Surrounding whitespace is trimmed exactly as the lower bound does.
        assert_eq!(requirement_upper_bound("  8.6-9.0  "), Some("9.0"));
        assert_eq!(requirement_lower_bound("  8.6-9.0  "), "8.6");
    }
}
