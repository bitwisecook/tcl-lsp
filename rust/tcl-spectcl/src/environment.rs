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

//! The pack loader's dialect **name** ingress — `tcl-spectcl`'s face of
//! the one shared seam, [`tcl_registry::model::ingress`] (centralisation
//! contract R-a; P1-F wave 4).
//!
//! Only the name ingress lives here. Everything else in this crate keys off
//! an already-resolved [`DialectProfile`] or off a pack's own declarations,
//! and moves with the profile when ledger C1's re-type retires it
//! (post-P1-G, which deleted the name validators).
//!
//! The pack-carrying registries this crate publishes are deliberately
//! resolved through the **analyser** profile
//! ([`DocumentEnvironment::analyser_profile`], the exact
//! `DialectProfile::by_name` twin) rather than the promoting document form:
//! the pack overlay is cached on `(profile, key)`, so promoting `tk` to its
//! own profile here would split a cache entry that has always been the
//! plain-Tcl one.
//!
//! [`DocumentEnvironment::analyser_profile`]: tcl_registry::model::DocumentEnvironment::analyser_profile

use tcl_dialect::DialectProfile;

/// Resolve a dialect **name** to the profile a pack-carrying registry is
/// built for — the exact environment-model twin of
/// `DialectProfile::by_name`: an alias canonicalises, and an unknown
/// spelling lands on the permissive fallback exactly as it always did, so
/// a stream of typos cannot leak one registry per typo.
#[must_use]
pub fn profile_for_dialect(name: &str) -> &'static DialectProfile {
    tcl_registry::model::resolve_environment(name).analyser_profile()
}

/// The **catalogue** profile a dialect name has, `None` when it names no
/// catalogue entry — the environment-model form of `DialectProfile::find`,
/// for the two readers that must render *nothing* rather than the
/// fallback's placeholder: a `file_extension -dialect` routing claim and
/// the studio's dialect labels.
#[must_use]
pub fn catalogue_profile_for_dialect(name: &str) -> Option<&'static DialectProfile> {
    tcl_registry::model::resolve_environment(name).catalogue_profile()
}

/// The **lenient** command store — the permissive all-Tcl view the
/// collision policy consults, replacing
/// the retired `registry_for_profile` at the permissive fallback profile.
#[must_use]
pub fn lenient_store() -> &'static tcl_registry::CommandRegistry {
    tcl_registry::model::static_context_for("tcl").commands()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_ingress_reproduces_the_retired_validators() {
        for profile in DialectProfile::all() {
            assert!(std::ptr::eq(profile_for_dialect(profile.name), profile));
            assert_eq!(
                catalogue_profile_for_dialect(profile.name).map(|p| p.name),
                Some(profile.name)
            );
            for &alias in profile.aliases {
                assert!(std::ptr::eq(profile_for_dialect(alias), profile), "{alias}");
            }
        }
        // The names with no catalogue entry: the lenient sink, the additive
        // `tk` library surface, and anything unknown.
        for name in ["", "tcl", "tk", "not-a-real-dialect"] {
            assert!(profile_for_dialect(name).is_fallback(), "{name}");
            assert!(catalogue_profile_for_dialect(name).is_none(), "{name}");
        }
    }
}
