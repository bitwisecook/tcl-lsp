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

//! The explorer's dialect ingress — its face of the one shared seam,
//! [`tcl_registry::model::ingress`].
//!
//! Every dialect **name** the explorer pipeline accepts — `run_pipeline`'s
//! `dialect` argument, and each serialiser view's `result.dialect` —
//! resolves here, once. Nothing here changes what a view reports: the
//! catalogue names the explorer resolves map to their same-named
//! environments, whose profiles are exactly the ones `DialectProfile`'s own
//! name-resolution methods return for the same string.
//!
//! Four resolver forms, because the explorer needs four:
//!
//! * [`profile_for_dialect`] is the *promoting* form — it tries
//!   `resolve_known(name)` first, falling back to `by_name(name)`, the
//!   ingress `run_pipeline` uses for its dialect argument — so `tk` keeps
//!   the typed additive profile;
//! * [`analyser_profile_for_dialect`] is the exact `DialectProfile::by_name`
//!   twin, for the one reader (`serialise_event_order`'s head-identity
//!   resolution) that took the plain fallback for `tk`;
//! * [`known_profile_for_dialect`] is the exact `DialectProfile::resolve_known`
//!   twin — `Option`-returning, promoting `tk` — for the serialiser views
//!   that thread an optional profile into the GVN/taint/optimiser/irules-flow
//!   finders and answer `None`'s availability mask as empty rather than the
//!   fallback's;
//! * [`catalogue_profile_for_dialect`] is the exact `DialectProfile::find`
//!   twin — `Option`-returning, **not** promoting `tk` — for the two readers
//!   (`serialise_meta`'s dialect labels, `serialise_bounds`'s character-model
//!   lookup) that must render nothing for a name with no catalogue entry
//!   rather than serve `tk`'s additive profile.

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

/// Resolve a dialect **name** only when it names a real environment — the
/// exact `DialectProfile::resolve_known` twin: `Option`-returning, and
/// (unlike [`catalogue_profile_for_dialect`]) promoting `tk` to its typed
/// additive profile.
///
/// The serialiser views that thread `Option<&DialectProfile>` into the GVN /
/// taint / optimiser / irules-flow finders read it here; a `None` still maps
/// to `None` at each `surface_query` read, not to the fallback's permissive
/// mask, exactly as `DialectProfile::resolve_known` requires.
///
/// Deliberately **not** `resolve_known_environment(name).map(unit_profile)`:
/// that seam helper answers `Some(&PLAIN_TCL)` for the literal name `"tcl"`
/// (a registered environment id, for the editor-identity and lenient-sink
/// roles), where `DialectProfile::resolve_known("tcl")` —
/// `find("tcl").or_else(|| tcl_dialect::DialectProfile::find("tcl").map(tcl_dialect::DialectProfile::surface_query)…)`, and `"tcl"` is
/// neither a catalogue profile name nor a `DialectProfile::find` spelling —
/// answers `None`. Composed from [`catalogue_profile_for_dialect`] (the
/// exact `find` twin) plus the one `DialectProfile::find` promotion this
/// contract requires (`"tk"`) instead, so this answers `None` for `"tcl"`
/// exactly as that contract does. Confirmed by a direct probe against both
/// APIs, not merely by re-reading the source.
#[must_use]
pub fn known_profile_for_dialect(name: &str) -> Option<&'static DialectProfile> {
    catalogue_profile_for_dialect(name)
        .or_else(|| (name == "tk").then(|| profile_for_dialect("tk")))
}

/// Resolve a dialect **name** only when it names a real **catalogue**
/// entry — the exact `DialectProfile::find` twin: `Option`-returning, and
/// (unlike [`known_profile_for_dialect`]) never promoting `tk`, because `tk`
/// is a library placement rather than a catalogue profile.
///
/// `serialise_meta`'s per-dialect label lookup and `serialise_bounds`'s
/// character-model lookup (fed a unit's own semantic dialect, which *can*
/// name `tk`) both need the pre-promotion
/// answer: `None` for `tk`, exactly as `DialectProfile::find("tk")` always
/// has.
#[must_use]
pub fn catalogue_profile_for_dialect(name: &str) -> Option<&'static DialectProfile> {
    tcl_registry::model::resolve_environment(name).catalogue_profile()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The four ingress forms differ at exactly one name — `tk` — and agree
    /// everywhere else the explorer can reach.
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
                known_profile_for_dialect(name).expect(name)
            ));
            assert!(std::ptr::eq(
                profile_for_dialect(name),
                catalogue_profile_for_dialect(name).expect(name)
            ));
        }
        for name in ["", "tcl", "not-a-real-dialect"] {
            assert!(profile_for_dialect(name).is_fallback(), "{name}");
            assert!(analyser_profile_for_dialect(name).is_fallback(), "{name}");
            assert!(known_profile_for_dialect(name).is_none(), "{name}");
            assert!(catalogue_profile_for_dialect(name).is_none(), "{name}");
        }
        assert_eq!(profile_for_dialect("tk").name, "tk");
        assert!(analyser_profile_for_dialect("tk").is_fallback());
        assert!(known_profile_for_dialect("tk").is_some_and(|p| p.name == "tk"));
        assert!(catalogue_profile_for_dialect("tk").is_none());
    }
}
