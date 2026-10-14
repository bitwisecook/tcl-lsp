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

//! Explorer dialect ingress through the shared environment model.
//!
//! Pipeline compilation uses the unit profile. Analyser queries use the
//! separately selected analyser profile, which retains the permissive `tk`
//! fallback.

use tcl_dialect::DialectProfile;

/// Resolve a dialect **name** to the profile the explorer pipeline builds
/// its compilation unit against — the environment-model form of
/// `resolve_known(name).unwrap_or_else(|| by_name(name))`, the ingress
/// [`crate::run_pipeline`] uses.
#[must_use]
pub fn profile_for_dialect(name: &str) -> &'static DialectProfile {
    tcl_registry::model::resolve_environment(name).unit_profile()
}

/// Resolve a dialect **name** to the profile the *analyser* threads — the
/// exact environment-model twin of `DialectProfile::by_name`, which sinks
/// `tk` to the permissive fallback rather than promoting it.
///
/// Distinct from [`profile_for_dialect`] at exactly one name, `tk`, and kept
/// distinct so `serialise_event_order`'s head-identity resolution — which
/// took the fallback's answer for `tk` — keeps taking it.
#[must_use]
pub fn analyser_profile_for_dialect(name: &str) -> &'static DialectProfile {
    tcl_registry::model::resolve_environment(name).analyser_profile()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ingress_forms_differ_only_at_tk() {
        for profile in DialectProfile::all() {
            let name = profile.name;
            assert!(std::ptr::eq(
                profile_for_dialect(name),
                analyser_profile_for_dialect(name)
            ));
            assert!(std::ptr::eq(
                profile_for_dialect(name),
                tcl_registry::model::resolve_environment(name)
                    .catalogue_profile()
                    .or_else(|| (name == "tk").then(|| profile_for_dialect(name)))
                    .expect(name)
            ));
            assert!(std::ptr::eq(
                profile_for_dialect(name),
                tcl_registry::model::resolve_environment(name)
                    .catalogue_profile()
                    .expect(name)
            ));
        }
        for name in ["", "tcl", "not-a-real-dialect"] {
            assert!(profile_for_dialect(name).is_fallback(), "{name}");
            assert!(analyser_profile_for_dialect(name).is_fallback(), "{name}");
            assert!(
                tcl_registry::model::resolve_environment(name)
                    .catalogue_profile()
                    .or_else(|| (name == "tk").then(|| profile_for_dialect(name)))
                    .is_none(),
                "{name}"
            );
            assert!(
                tcl_registry::model::resolve_environment(name)
                    .catalogue_profile()
                    .is_none(),
                "{name}"
            );
        }
        assert_eq!(profile_for_dialect("tk").name, "tk");
        assert!(analyser_profile_for_dialect("tk").is_fallback());
        assert!(
            tcl_registry::model::resolve_environment("tk")
                .catalogue_profile()
                .or_else(|| Some(tcl_registry::model::resolve_environment("tk").unit_profile()))
                .is_some_and(|p| p.name == "tk")
        );
        assert!(
            tcl_registry::model::resolve_environment("tk")
                .catalogue_profile()
                .is_none()
        );
    }
}
