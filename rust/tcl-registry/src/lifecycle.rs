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

//! One lifecycle contract for every versioned registry entity.
//!
//! Commands, subcommands, options, literal argument values, iRules events,
//! and `BIG-IP` profile types all age the same way, so they all carry the same
//! three-release fact family:
//!
//! * [`Lifecycle::introduced`] — the first release where the item exists;
//! * [`Lifecycle::deprecated`] — the first release where it still exists but
//!   should warn;
//! * [`Lifecycle::retired`] — the first release where it no longer exists
//!   (an **exclusive** upper boundary).
//!
//! An absent release means the lifecycle has not reached that state. There is
//! deliberately no generic "maximum version": a known-good range is described
//! by [`Lifecycle::introduced`] alone, and an upper bound exists *only* as
//! retirement metadata.
//!
//! Availability is `introduced <= target < retired`. Deprecation applies only
//! while the item is still available, i.e. `deprecated <= target < retired`.
//! The exclusive retirement rule is written once, here, so every lookup,
//! completion, diagnostic, and export agrees.
//!
//! An absent *target* version (the caller could not resolve one) is
//! permissive: the item is treated as available and not deprecated, matching
//! the registry's long-standing "no version known ⇒ do not gate" behaviour.

use crate::version::compare;

/// Where a versioned registry entity sits relative to a target release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LifecycleState {
    /// The target release predates [`Lifecycle::introduced`].
    NotIntroduced,
    /// Available, with no deprecation declared or not yet reached.
    Available,
    /// Available but deprecated at the target release.
    Deprecated,
    /// The target release is at or past [`Lifecycle::retired`].
    Retired,
}

impl LifecycleState {
    /// Whether the entity exists at all at the target release.
    #[must_use]
    pub const fn is_available(self) -> bool {
        matches!(self, Self::Available | Self::Deprecated)
    }

    /// Stable lower-case identifier, used by CLI/MCP/JSON surfaces.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotIntroduced => "not-introduced",
            Self::Available => "available",
            Self::Deprecated => "deprecated",
            Self::Retired => "retired",
        }
    }
}

/// An impossible lifecycle ordering, as reported by [`Lifecycle::validate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleError {
    /// `deprecated` predates `introduced`.
    DeprecatedBeforeIntroduced,
    /// `retired` predates `introduced`.
    RetiredBeforeIntroduced,
    /// `retired` predates `deprecated`.
    RetiredBeforeDeprecated,
}

impl LifecycleError {
    /// Human-readable explanation for validation failures.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::DeprecatedBeforeIntroduced => {
                "deprecated release predates the introducing release"
            }
            Self::RetiredBeforeIntroduced => "retired release predates the introducing release",
            Self::RetiredBeforeDeprecated => "retired release predates the deprecating release",
        }
    }
}

impl std::fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

/// Introduction / deprecation / retirement releases for one registry entity.
///
/// All three releases are dotted version strings on the entity's own version
/// axis — the Tcl core version, an owning package's version (Tk, tcllib,
/// tcltest), or the `BIG-IP` release, depending on what carries the lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Lifecycle {
    /// First release where the entity exists. `None` = present in every
    /// release of its axis.
    pub introduced: Option<&'static str>,
    /// First release where the entity is still available but deprecated.
    /// `None` = not deprecated.
    pub deprecated: Option<&'static str>,
    /// First release where the entity no longer exists — **exclusive**.
    /// `None` = not retired.
    pub retired: Option<&'static str>,
}

impl Lifecycle {
    /// No lifecycle facts declared: always available, never deprecated.
    pub const UNSPECIFIED: Self = Self {
        introduced: None,
        deprecated: None,
        retired: None,
    };

    /// Introduced in `release`, with no deprecation or retirement.
    #[must_use]
    pub const fn introduced_in(release: &'static str) -> Self {
        Self {
            introduced: Some(release),
            deprecated: None,
            retired: None,
        }
    }

    /// Deprecated from `release` (still available).
    #[must_use]
    pub const fn deprecated_in(release: &'static str) -> Self {
        Self {
            introduced: None,
            deprecated: Some(release),
            retired: None,
        }
    }

    /// This lifecycle with a deprecation release added.
    #[must_use]
    pub const fn deprecated_from(mut self, release: &'static str) -> Self {
        self.deprecated = Some(release);
        self
    }

    /// This lifecycle with a retirement release added (exclusive: `release`
    /// is the first release that no longer has the entity).
    #[must_use]
    pub const fn retired_from(mut self, release: &'static str) -> Self {
        self.retired = Some(release);
        self
    }

    /// Whether any lifecycle release is declared.
    #[must_use]
    pub const fn is_unspecified(&self) -> bool {
        self.introduced.is_none() && self.deprecated.is_none() && self.retired.is_none()
    }

    /// The lifecycle state at `target`. `None` (unknown target release) is
    /// permissive and yields [`LifecycleState::Available`].
    #[must_use]
    pub fn state_at(&self, target: Option<&str>) -> LifecycleState {
        let Some(target) = target else {
            return LifecycleState::Available;
        };
        if let Some(retired) = self.retired
            && compare(target, retired).is_ge()
        {
            return LifecycleState::Retired;
        }
        if let Some(introduced) = self.introduced
            && compare(target, introduced).is_lt()
        {
            return LifecycleState::NotIntroduced;
        }
        if let Some(deprecated) = self.deprecated
            && compare(target, deprecated).is_ge()
        {
            return LifecycleState::Deprecated;
        }
        LifecycleState::Available
    }

    /// Whether the entity exists at `target` — `introduced <= target < retired`.
    #[must_use]
    pub fn available_at(&self, target: Option<&str>) -> bool {
        self.state_at(target).is_available()
    }

    /// Whether the entity is deprecated *and still available* at `target`.
    #[must_use]
    pub fn deprecated_at(&self, target: Option<&str>) -> bool {
        self.state_at(target) == LifecycleState::Deprecated
    }

    /// Whether the entity is retired at `target`.
    #[must_use]
    pub fn retired_at(&self, target: Option<&str>) -> bool {
        self.state_at(target) == LifecycleState::Retired
    }

    /// Whether the entity is *ever* deprecated, independent of any target
    /// release. Used by version-agnostic surfaces (a plain hover with no
    /// resolved dialect version) that previously read a boolean flag.
    #[must_use]
    pub const fn is_deprecated_anywhere(&self) -> bool {
        self.deprecated.is_some() || self.retired.is_some()
    }

    /// Reject impossible orderings. Registry sweep tests run this over every
    /// entity so bad data cannot reach a consumer.
    ///
    /// # Errors
    /// Returns the first ordering violation found.
    pub fn validate(&self) -> Result<(), LifecycleError> {
        if let (Some(introduced), Some(deprecated)) = (self.introduced, self.deprecated)
            && compare(deprecated, introduced).is_lt()
        {
            return Err(LifecycleError::DeprecatedBeforeIntroduced);
        }
        if let (Some(introduced), Some(retired)) = (self.introduced, self.retired)
            && compare(retired, introduced).is_lt()
        {
            return Err(LifecycleError::RetiredBeforeIntroduced);
        }
        if let (Some(deprecated), Some(retired)) = (self.deprecated, self.retired)
            && compare(retired, deprecated).is_lt()
        {
            return Err(LifecycleError::RetiredBeforeDeprecated);
        }
        Ok(())
    }

    /// This lifecycle with an axis baseline substituted for an absent
    /// introducing release — the `BIG-IP` event axis declares everything
    /// present since its baseline unless it says otherwise.
    ///
    /// The baseline is an *assumption*, not a fact, so it never overrides a
    /// declared release and never creates an impossible ordering: an entity
    /// deprecated or retired before the baseline keeps an open introduction
    /// rather than claiming to have appeared after it was already going away.
    #[must_use]
    pub fn with_baseline(mut self, baseline: Option<&'static str>) -> Self {
        if self.introduced.is_some() {
            return self;
        }
        let Some(baseline) = baseline else {
            return self;
        };
        let precedes_baseline =
            |release: Option<&str>| release.is_some_and(|r| compare(r, baseline).is_lt());
        if precedes_baseline(self.deprecated) || precedes_baseline(self.retired) {
            return self;
        }
        self.introduced = Some(baseline);
        self
    }

    /// Narrow to the stricter of two lifecycles: the later introduction and
    /// the earlier deprecation/retirement. Used where an entity inherits a
    /// parent's gate (a subcommand of a package-gated command).
    #[must_use]
    pub fn intersect(self, other: Self) -> Self {
        fn later(a: Option<&'static str>, b: Option<&'static str>) -> Option<&'static str> {
            match (a, b) {
                (Some(a), Some(b)) => Some(if compare(a, b).is_ge() { a } else { b }),
                (some, None) | (None, some) => some,
            }
        }
        fn earlier(a: Option<&'static str>, b: Option<&'static str>) -> Option<&'static str> {
            match (a, b) {
                (Some(a), Some(b)) => Some(if compare(a, b).is_le() { a } else { b }),
                (some, None) | (None, some) => some,
            }
        }
        Self {
            introduced: later(self.introduced, other.introduced),
            deprecated: earlier(self.deprecated, other.deprecated),
            retired: earlier(self.retired, other.retired),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLASSIC_XML: Lifecycle = Lifecycle::introduced_in("9.0.3")
        .deprecated_from("10.0.0")
        .retired_from("10.0.0");

    #[test]
    fn unspecified_is_always_available() {
        let life = Lifecycle::UNSPECIFIED;
        assert!(life.is_unspecified());
        assert_eq!(life.state_at(Some("8.4")), LifecycleState::Available);
        assert_eq!(life.state_at(None), LifecycleState::Available);
        assert!(!life.deprecated_at(Some("9.0")));
    }

    #[test]
    fn introduction_gates_the_lower_bound() {
        let life = Lifecycle::introduced_in("8.5");
        assert_eq!(life.state_at(Some("8.4")), LifecycleState::NotIntroduced);
        assert!(!life.available_at(Some("8.4")));
        assert_eq!(life.state_at(Some("8.5")), LifecycleState::Available);
        assert_eq!(life.state_at(Some("9.0")), LifecycleState::Available);
        // Unknown target release stays permissive.
        assert!(life.available_at(None));
    }

    #[test]
    fn retirement_is_exclusive() {
        // Classic XML parser events: present from 9.0.3, unavailable from
        // 10.0.0 — so 9.4.8 still has them and 10.0.0 itself does not.
        assert_eq!(
            CLASSIC_XML.state_at(Some("9.0.2")),
            LifecycleState::NotIntroduced
        );
        assert_eq!(
            CLASSIC_XML.state_at(Some("9.0.3")),
            LifecycleState::Available
        );
        assert_eq!(
            CLASSIC_XML.state_at(Some("9.4.8")),
            LifecycleState::Available
        );
        assert_eq!(
            CLASSIC_XML.state_at(Some("10.0.0")),
            LifecycleState::Retired
        );
        assert_eq!(
            CLASSIC_XML.state_at(Some("11.0.0")),
            LifecycleState::Retired
        );
        assert!(!CLASSIC_XML.available_at(Some("10.0.0")));
    }

    #[test]
    fn deprecation_only_applies_while_available() {
        // AUTH_* events: introduced 9.0.0, deprecated 9.4.0, never retired.
        let auth = Lifecycle::introduced_in("9.0.0").deprecated_from("9.4.0");
        assert_eq!(auth.state_at(Some("9.3.0")), LifecycleState::Available);
        assert_eq!(auth.state_at(Some("9.4.0")), LifecycleState::Deprecated);
        assert_eq!(auth.state_at(Some("17.0.0")), LifecycleState::Deprecated);
        assert!(auth.available_at(Some("17.0.0")));
        // Retirement outranks deprecation.
        assert!(!CLASSIC_XML.deprecated_at(Some("10.0.0")));
        assert!(CLASSIC_XML.retired_at(Some("10.0.0")));
    }

    #[test]
    fn retired_before_introduced_is_not_available_anywhere_in_between() {
        let life = Lifecycle::introduced_in("11.5.0").deprecated_from("15.0.0");
        assert!(!life.available_at(Some("11.4.0")));
        assert_eq!(life.state_at(Some("15.0.0")), LifecycleState::Deprecated);
    }

    #[test]
    fn validation_rejects_impossible_orderings() {
        assert_eq!(
            Lifecycle::introduced_in("9.0")
                .deprecated_from("8.6")
                .validate(),
            Err(LifecycleError::DeprecatedBeforeIntroduced)
        );
        assert_eq!(
            Lifecycle::introduced_in("9.0")
                .retired_from("8.6")
                .validate(),
            Err(LifecycleError::RetiredBeforeIntroduced)
        );
        assert_eq!(
            Lifecycle::deprecated_in("9.0")
                .retired_from("8.6")
                .validate(),
            Err(LifecycleError::RetiredBeforeDeprecated)
        );
        // Deprecated == retired in the same release is legal (the classic XML
        // events were documented deprecated and unavailable at 10.0.0).
        assert_eq!(CLASSIC_XML.validate(), Ok(()));
        assert_eq!(Lifecycle::UNSPECIFIED.validate(), Ok(()));
    }

    #[test]
    fn baseline_fills_only_an_absent_introduction() {
        assert_eq!(
            Lifecycle::UNSPECIFIED
                .with_baseline(Some("15.0"))
                .introduced,
            Some("15.0")
        );
        assert_eq!(
            Lifecycle::introduced_in("9.0.3")
                .with_baseline(Some("15.0"))
                .introduced,
            Some("9.0.3")
        );
    }

    #[test]
    fn intersect_takes_the_stricter_bound_on_each_side() {
        let parent = Lifecycle::introduced_in("8.5").retired_from("9.1");
        let child = Lifecycle::introduced_in("8.6").deprecated_from("9.0");
        let merged = parent.intersect(child);
        assert_eq!(merged.introduced, Some("8.6"));
        assert_eq!(merged.deprecated, Some("9.0"));
        assert_eq!(merged.retired, Some("9.1"));
    }

    #[test]
    fn state_names_are_stable() {
        assert_eq!(LifecycleState::NotIntroduced.as_str(), "not-introduced");
        assert_eq!(LifecycleState::Available.as_str(), "available");
        assert_eq!(LifecycleState::Deprecated.as_str(), "deprecated");
        assert_eq!(LifecycleState::Retired.as_str(), "retired");
    }
}
