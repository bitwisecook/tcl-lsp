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

//! The CLIs' dialect ingress — the `tcl` / `f5` face of the one shared
//! seam, [`tcl_registry::model::ingress`] (centralisation contract R-a;
//! P1-F wave 4, alongside the MCP server and the spec studio).
//!
//! Every dialect **name** a CLI verb accepts — an explicit `--dialect`, a
//! detector verdict, the `tcl8.6` invocation default, the fixed
//! `f5-irules` the iRule verbs simulate — resolves here, once, and the
//! availability questions the verb then asks are answered by the resolved
//! environment's [`ResolvedContext`] rather than by `ProfileQueries` over
//! a threaded profile.
//!
//! Nothing here changes what a verb prints. The catalogue names the CLI
//! resolves map to their same-named environments, whose
//! [`unit_profile`](tcl_registry::model::DocumentEnvironment::unit_profile)
//! is the profile the retired validators returned, and whose document
//! context answers availability under the **document authoring mask** —
//! test-pinned equal to the threaded profile's `availability_mask` for
//! every profile an ingress can produce.
//!
//! Two ingress forms, deliberately distinct, because the CLI used both:
//!
//! * [`profile_for_dialect`] is the *promoting* form (the old
//!   `resolve_known(name).unwrap_or_else(|| by_name(name))` ingress), so
//!   `tk` keeps the typed additive profile the CLI's `--dialect tk`
//!   resolves to;
//! * [`analyser_profile_for_dialect`] is the exact `DialectProfile::by_name`
//!   twin, for the readers that deliberately took the permissive fallback
//!   for `tk` (the KCS help filter's term set).

use tcl_dialect::DialectProfile;
use tcl_registry::model::ResolvedContext;

/// Resolve a dialect **name** to the profile a CLI verb threads — the
/// environment-model form of the CLI's `resolve_known`-then-`by_name`
/// ingress and of the named constructors (`plain_tcl`, `irules`, `tk`).
///
/// P1-G: the profile retires and the verbs read their labels and grammar
/// facts off the environment instead.
#[must_use]
pub fn profile_for_dialect(name: &str) -> &'static DialectProfile {
    tcl_registry::model::resolve_environment(name).unit_profile()
}

/// Resolve a dialect **name** to the profile the *analyser* threads — the
/// exact environment-model twin of `DialectProfile::by_name`, which sinks
/// `tk` to the permissive fallback rather than promoting it.
///
/// Distinct from [`profile_for_dialect`] at exactly one name, `tk`, and
/// kept distinct so the readers that took the fallback's answer for `tk`
/// (the KCS help filter's empty term set) keep taking it.
#[must_use]
pub fn analyser_profile_for_dialect(name: &str) -> &'static DialectProfile {
    tcl_registry::model::resolve_environment(name).analyser_profile()
}

/// Resolve a dialect **name** only when it names a real environment — the
/// validator form, replacing `DialectProfile::resolve_known(name)` at the
/// CLI ingest boundary, where an unrecognised spelling must be an input
/// error rather than a silent fallback to plain Tcl (ledger rows T1/T5).
#[must_use]
pub fn known_profile_for_dialect(name: &str) -> Option<&'static DialectProfile> {
    tcl_registry::model::resolve_known_environment(name)
        .map(|environment| environment.unit_profile())
}

/// The **document context** a dialect name's answers are given under — the
/// assistance view that replaces the whole `ProfileQueries` surface
/// (ledger row F1's assistance half): command resolution, availability,
/// subcommands, options.
#[must_use]
pub fn context_for_dialect(name: &str) -> &'static ResolvedContext {
    tcl_registry::model::static_document_context_for(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two ingress forms differ at exactly one name — the documented
    /// `tk` promotion — and agree everywhere else the CLI can reach.
    #[test]
    fn the_two_ingress_forms_differ_only_at_tk() {
        for profile in DialectProfile::all() {
            assert!(std::ptr::eq(
                profile_for_dialect(profile.name),
                analyser_profile_for_dialect(profile.name)
            ));
        }
        for name in ["", "tcl", "not-a-real-dialect"] {
            assert!(std::ptr::eq(
                profile_for_dialect(name),
                analyser_profile_for_dialect(name)
            ));
            assert!(profile_for_dialect(name).is_fallback(), "{name}");
        }
        assert_eq!(profile_for_dialect("tk").name, "tk");
        assert!(analyser_profile_for_dialect("tk").is_fallback());
    }

    /// The document context a verb answers under carries the very mask the
    /// threaded profile does, for every name a CLI ingress can produce.
    #[test]
    fn the_document_context_carries_the_threaded_masks() {
        let names: Vec<&str> = DialectProfile::all()
            .iter()
            .map(|profile| profile.name)
            .chain(["tk", "tcl", "", "irules", "not-a-real-dialect"])
            .collect();
        for name in names {
            assert_eq!(
                context_for_dialect(name).authoring_mask(),
                profile_for_dialect(name).availability_mask,
                "{name}"
            );
        }
    }
}
