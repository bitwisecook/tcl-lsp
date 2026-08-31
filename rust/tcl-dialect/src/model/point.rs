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

//! The resolved dialect **point** — one place on one family's ladder, and the
//! currency a layer should carry when it needs to know "which Tcl is this".
//!
//! A [`Release`] already names its [`Family`], so a point is a release plus
//! the build profile, and every lexical axis is a function of it
//! ([`Self::grammar`]). That is the whole model P6 moved to: the grammar is a
//! function of `(family, release, build)` rather than a row in a catalogue.
//!
//! It exists because the two currencies layers actually carried could not say
//! what they needed to:
//!
//! * **`Option<&DialectProfile>`** cannot name a dialect with no catalogue
//!   profile. `jim` and `tk` have none — deliberately, since P6 — so every
//!   `of_profile` call answered C Tcl for a Jim document, and codegen
//!   compiled it with 9.0 numerals and escapes.
//! * **`Option<&str>`** (a dialect *name*) resolves only to an environment's
//!   *default* release, so it cannot distinguish the minor versions that
//!   actually differ: jim 0.79 from 0.80 (`0d` numerals), tcl8.5 from 8.6
//!   (TIP 388 escape widths), tcl9.0 from 9.1.
//!
//! A point says both. `DialectPoint::of_name_and_release(Some("jim"), Some(
//! Release::JIM_0_79))` is a different grammar from the same name at 0.84,
//! and `Some("tcl8.5")` differs from `Some("tcl8.6")` — which is the whole
//! reason the escape ladder has separate rungs.

use crate::LexerGrammar;
use crate::model::{BuildProfileId, Family, Release, grammar};

/// One resolved point on one family's ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DialectPoint {
    release: Release,
    build: BuildProfileId,
}

impl Default for DialectPoint {
    /// Modern Tcl — the long-standing "no dialect means the default rule".
    fn default() -> Self {
        Self {
            release: Release::TCL_9_0,
            build: BuildProfileId::Canonical,
        }
    }
}

impl DialectPoint {
    /// A point on `release`'s ladder, at `build`.
    #[must_use]
    pub const fn new(release: Release, build: BuildProfileId) -> Self {
        Self { release, build }
    }

    /// A point at the canonical build — the common case.
    #[must_use]
    pub const fn canonical(release: Release) -> Self {
        Self::new(release, BuildProfileId::Canonical)
    }

    /// The family this point sits on.
    #[must_use]
    pub const fn family(self) -> Family {
        self.release.family()
    }

    /// The exact release — the minor version a name alone cannot carry.
    #[must_use]
    pub const fn release(self) -> Release {
        self.release
    }

    /// The build profile, which capability questions read.
    #[must_use]
    pub const fn build(self) -> BuildProfileId {
        self.build
    }

    /// Every lexical axis of this point, in one value.
    ///
    /// The single derivation: a consumer takes the grammar from here rather
    /// than resolving one axis at a time, so two axes cannot come from
    /// different points for the same document.
    #[must_use]
    pub const fn grammar(self) -> LexerGrammar {
        grammar(self.release.family(), self.release)
    }

    /// The point a dialect *name* selects, at that environment's default
    /// release.
    ///
    /// `None` when the name names no Tcl ladder at all. That is not a failure
    /// mode but a real case: `f5-bigip` is the BIG-IP *config* dialect, which
    /// the model records as having no core because it is not Tcl (§11.1's
    /// `Ternary::Inert`). It is the only such name among the shipped
    /// dialects — the environment model point-resolves the other seventeen.
    ///
    /// Use [`Self::of_name_and_release`] when the document pins a release,
    /// since this one takes the environment's default and so cannot tell jim
    /// 0.79 from 0.84.
    #[must_use]
    pub fn of_dialect_name(name: Option<&str>) -> Option<Self> {
        let name = name?;
        let definition = compiled_environments().resolve(name)?;
        let core = definition.core?;
        Some(Self::new(core.default_release, core.build))
    }

    /// The point a name selects at a *pinned* release — the form that keeps a
    /// minor version, and the one a document with `# tcl-lsp: supports jim
    /// 0.79` needs.
    ///
    /// A release from another family is not on this name's ladder, so it is
    /// ignored in favour of the environment's default rather than yielding a
    /// grammar the document was never lexed with.
    #[must_use]
    pub fn of_name_and_release(name: Option<&str>, release: Option<Release>) -> Option<Self> {
        let base = Self::of_dialect_name(name)?;
        Some(match release {
            Some(r) if r.family() == base.family() => Self::new(r, base.build),
            _ => base,
        })
    }
}

/// The compiled environment registry, built once.
///
/// Every name→point resolution reads it, so it is built one time rather than
/// per call: [`EnvironmentRegistry::compiled`] rebuilds the whole catalogue.
pub(crate) fn compiled_environments() -> &'static crate::model::EnvironmentRegistry {
    static REGISTRY: std::sync::LazyLock<crate::model::EnvironmentRegistry> =
        std::sync::LazyLock::new(crate::model::EnvironmentRegistry::compiled);
    &REGISTRY
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A point carries the *minor version*, which is the whole reason it
    /// exists: a name alone collapses each family to one release.
    #[test]
    fn a_point_distinguishes_releases_a_name_cannot() {
        let name_only = DialectPoint::of_dialect_name(Some("jim")).expect("jim has a core");
        let pinned = DialectPoint::of_name_and_release(Some("jim"), Some(Release::JIM_0_79))
            .expect("jim has a core");

        assert_eq!(name_only.release(), Release::JIM_0_84);
        assert_eq!(pinned.release(), Release::JIM_0_79);
        // 0.80 added `0d`, so the two rungs are different grammars — the
        // difference a name-only resolution silently loses.
        assert_ne!(name_only.grammar().numbers, pinned.grammar().numbers);
        assert_eq!(name_only.family(), Family::Jim);
        assert_eq!(pinned.family(), Family::Jim);
    }

    /// The same holds on the Tcl ladder, where TIP 388 moved the escape
    /// widths between 8.5 and 8.6.
    #[test]
    fn the_tcl_ladder_keeps_its_minor_versions() {
        let five = DialectPoint::of_dialect_name(Some("tcl8.5")).expect("core");
        let six = DialectPoint::of_dialect_name(Some("tcl8.6")).expect("core");
        assert_ne!(five.grammar().escapes, six.grammar().escapes);
        assert_eq!(five.family(), Family::Tcl);
    }

    /// A release from another family cannot be pinned onto this ladder.
    #[test]
    fn a_release_from_another_ladder_is_ignored() {
        let point =
            DialectPoint::of_name_and_release(Some("jim"), Some(Release::TCL_8_6)).expect("core");
        assert_eq!(point.release(), Release::JIM_0_84, "the jim default stands");
    }

    /// Every shipped dialect resolves to a point except the one that is not
    /// Tcl at all.
    #[test]
    fn every_shipped_dialect_but_the_config_one_has_a_point() {
        for profile in crate::DialectProfile::all() {
            let point = DialectPoint::of_dialect_name(Some(profile.name));
            if profile.name == "f5-bigip" {
                assert!(point.is_none(), "f5-bigip is the config dialect, not Tcl");
            } else {
                assert!(
                    point.is_some(),
                    "{} should resolve to a point",
                    profile.name
                );
            }
        }
    }

    /// A point's grammar is the family/release function, so it agrees with
    /// the catalogue wherever the catalogue also has an answer.
    #[test]
    fn a_points_grammar_matches_the_catalogue() {
        for profile in crate::DialectProfile::all() {
            let Some(point) = DialectPoint::of_dialect_name(Some(profile.name)) else {
                continue;
            };
            assert_eq!(
                point.grammar(),
                profile.grammar,
                "`{}` disagrees",
                profile.name
            );
        }
    }
}
