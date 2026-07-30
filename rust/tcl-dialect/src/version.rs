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

/// Parse a dotted version into numeric components.
fn version_components(v: &str) -> Vec<u64> {
    v.split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

/// Compare two dotted versions component-wise (missing components are 0).
#[must_use]
pub fn compare_versions(a: &str, b: &str) -> core::cmp::Ordering {
    let (va, vb) = (version_components(a), version_components(b));
    for i in 0..va.len().max(vb.len()) {
        let x = va.get(i).copied().unwrap_or(0);
        let y = vb.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            core::cmp::Ordering::Equal => {}
            other => return other,
        }
    }
    core::cmp::Ordering::Equal
}

/// Does the concrete `version` satisfy one `package vsatisfies` requirement?
///
/// The three requirement forms Tcl accepts (`package(n)`, verified against
/// `tclsh8.6` and `tclsh9.0`):
///
/// | Written  | Means            | `8.6` | `9.0` |
/// |----------|------------------|-------|-------|
/// | `8.5`    | `[8.5, 9)` — up to but excluding the *next major* | yes | no  |
/// | `8.5-`   | `[8.5, ∞)`       | yes   | yes   |
/// | `8.5-9.0`| `[8.5, 9.0)`     | yes   | no    |
///
/// This is the single implementation shared by the bytecode VM's `package
/// vsatisfies` and the language server's `pkgIndex.tcl` guard evaluation, so
/// the two can never disagree about what a guard means.
#[must_use]
pub fn version_satisfies(version: &str, requirement: &str) -> bool {
    use core::cmp::Ordering;
    let requirement = requirement.trim();
    let (lo, hi) = if let Some((lo, hi)) = requirement.split_once('-') {
        let hi = hi.trim();
        (lo.trim(), (!hi.is_empty()).then(|| hi.to_owned()))
    } else {
        // Bare `X.Y` → the upper bound is the next major version.
        let major = version_components(requirement)
            .first()
            .copied()
            .unwrap_or(0);
        (requirement, Some(format!("{}", major + 1)))
    };
    if compare_versions(version, lo) == Ordering::Less {
        return false;
    }
    match hi {
        Some(hi) => compare_versions(version, &hi) == Ordering::Less,
        None => true,
    }
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
