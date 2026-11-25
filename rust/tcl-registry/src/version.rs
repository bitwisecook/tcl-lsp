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

//! Package version helpers for registry lifecycle bounds and source advice.

pub use tcl_dialect::{compare_versions as compare, version_satisfies as satisfies_one};

/// True when *available* version is at least *minimum* version.
#[must_use]
pub fn meets_min(available: &str, minimum: &str) -> bool {
    compare(available, minimum).is_ge()
}

/// Whether any requirement admits the supplied version. An empty requirement
/// list is an unconstrained package lookup.
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
    tcl_dialect::package_requirement_bounds_for(req, tcl_dialect::TclVersion::V9_1)
        .map_or(req, |(lower, _)| lower)
}

/// A lower bound shared by every alternative in one package requirement.
/// Unknown or invalid alternatives leave that bound undetermined.
#[must_use]
pub fn alternatives_lower_bound<'version>(
    alternatives: impl IntoIterator<Item = Option<&'version str>>,
    exact: bool,
) -> Option<&'version str> {
    alternatives_lower_bound_for(alternatives, exact, tcl_dialect::TclVersion::V9_1)
}

/// A source-advice lower bound under the selected release's package grammar.
#[must_use]
pub fn alternatives_lower_bound_for<'version>(
    alternatives: impl IntoIterator<Item = Option<&'version str>>,
    exact: bool,
    grammar: tcl_dialect::TclVersion,
) -> Option<&'version str> {
    let mut alternatives = alternatives.into_iter();
    if exact || !grammar.has_package_requirements() {
        let version = alternatives.next()??;
        return (alternatives.next().is_none()
            && tcl_dialect::validate_version_for(version, grammar))
        .then_some(version);
    }
    let mut floor = None;
    for alternative in alternatives {
        let (lower, _) = tcl_dialect::package_requirement_bounds_for(alternative?, grammar)?;
        if floor.is_none_or(|previous| {
            tcl_dialect::compare_versions_for(lower, previous, grammar).is_lt()
        }) {
            floor = Some(lower);
        }
    }
    floor
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
    tcl_dialect::package_requirement_bounds_for(requirement.trim(), tcl_dialect::TclVersion::V9_1)
        .and_then(|(_, upper)| upper)
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
        assert_eq!(compare("8.6a1", "8.6b1"), Less);
        assert_eq!(compare("8.6b1", "8.6"), Less);
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
