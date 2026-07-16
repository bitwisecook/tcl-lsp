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

//! Profile-aware availability queries over registry data.
//!
//! [`DialectProfile`] lives in the foundational `tcl-dialect` crate, below
//! this one, so it cannot know about [`CommandSpec`] / [`CommandRegistry`].
//! This extension trait gives the profile its availability API (design doc
//! §5.1) at the registry layer: **every** availability consumer resolves
//! commands through these methods so the subtractive iRules disable list
//! (§9) is applied uniformly after the mask query — a bare mask query
//! (`get_for_dialect` alone) would silently re-admit the banned commands
//! once the Milestone 5 data retag lands.

use tcl_dialect::DialectProfile;

use crate::registry::CommandRegistry;
use crate::spec::CommandSpec;

/// Availability queries a resolved [`DialectProfile`] answers against
/// registry data (design doc §5.1). Implemented for `DialectProfile` here
/// because the spec types live above the foundational crate.
pub trait ProfileQueries {
    /// Whether `spec` is available under this profile: the membership test
    /// against [`DialectProfile::availability_mask`] **and** the subtractive
    /// disable filter ([`DialectProfile::is_command_disabled`], §9).
    fn is_available(&self, spec: &CommandSpec) -> bool;

    /// Resolve `name` to its command spec under this profile — the single
    /// availability primitive the diagnostics (W123/W002), completion,
    /// and the CLI snapshot share. Mask query + disable filter.
    fn resolve_command<'r>(
        &self,
        registry: &'r CommandRegistry,
        name: &str,
    ) -> Option<&'r CommandSpec>;
}

impl ProfileQueries for DialectProfile {
    fn is_available(&self, spec: &CommandSpec) -> bool {
        spec.supports_dialect(self.availability_mask) && !self.is_command_disabled(spec.name)
    }

    fn resolve_command<'r>(
        &self,
        registry: &'r CommandRegistry,
        name: &str,
    ) -> Option<&'r CommandSpec> {
        registry
            .get_for_dialect(name, self.availability_mask)
            .filter(|spec| !self.is_command_disabled(spec.name))
    }
}
