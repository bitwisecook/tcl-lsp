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

//! The version floor a **request-time** provider answers at.
//!
//! Every lifecycle-bearing fact in the registry — a command, a subcommand, an
//! option, an enumerable value, and since issue #1644 a per-argument row — is
//! read through an accessor taking `package_version: Option<&str>`. The floor
//! itself is a per-document fact, so `tcl-registry` never holds one: its
//! handles are cached per (profile, pack overlay) and shared across documents.
//!
//! This is the provider-side half of that split, in one place so completion,
//! and any provider that follows it, cannot drift on what a document's floor
//! is. It is deliberately **request-time only**: the answer needs the whole
//! document's `package require` lines, which exist only once the walk that
//! records them has finished. A consumer running *during* the walk cannot use
//! this — that is why the arity gate defers its verdict to a post-walk flush
//! (issue #1627) rather than asking mid-walk.

use tcl_compiler::analyser::AnalysisResult;
use tcl_registry::profile_queries::ProfileQueries as _;

/// One document's resolved-floor context, as a request-time provider sees it.
///
/// Copy-cheap (two references), so a provider threading it through helpers
/// pays nothing for passing it by value.
#[derive(Clone, Copy)]
pub struct DocumentFloor<'a> {
    analysis: &'a AnalysisResult,
    profile: &'static tcl_dialect::DialectProfile,
}

impl<'a> DocumentFloor<'a> {
    /// Bind the floor context to one analysed document under one profile.
    #[must_use]
    pub fn new(
        analysis: &'a AnalysisResult,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Self {
        Self { analysis, profile }
    }

    /// The guaranteed-available version floor for `spec`'s owning package.
    ///
    /// The profile's library pin supplies the base floor (§7.1: the shipped Tk
    /// on a plain Tcl base, a keyed vendor surface at its D5 oldest-supported
    /// default); an explicit `package require` can only **raise** it. When
    /// several requires name the same package, the most restrictive (highest)
    /// lower bound wins.
    ///
    /// Only *unconditional* requires count: an optional probe
    /// (`catch {package require Tk 8.7}`, or a require inside an `if` arm)
    /// guarantees nothing on every path, so counting it would raise the floor
    /// and hide a gated option, value or argument that may not be there.
    ///
    /// `None` when the command is not package-gated, or the package was
    /// required without a version — permissive, matching every other
    /// lifecycle query.
    #[must_use]
    pub fn for_spec(&self, spec: &tcl_registry::CommandSpec) -> Option<&'a str> {
        let package = self
            .profile
            .keyed_pin_for(spec)
            .map(|pin| pin.package)
            .or_else(|| spec.owning_package())?;
        let require_floor = self
            .analysis
            .package_requires
            .iter()
            .filter(|req| req.name == package && !req.conditional)
            .filter_map(|req| req.version.as_deref())
            .map(tcl_registry::version::requirement_lower_bound)
            .max_by(|a, b| tcl_registry::version::compare(a, b));
        let pin_floor = self
            .profile
            .library_floor(package, &self.analysis.library_versions);
        match (pin_floor, require_floor) {
            (Some(pin), Some(required)) => {
                if tcl_registry::version::compare(required, pin).is_gt() {
                    Some(required)
                } else {
                    Some(pin)
                }
            }
            (pin, required) => pin.or(required),
        }
    }
}
