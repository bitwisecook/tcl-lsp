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

//! Explicit logical core simulation context, independent of native authority.

/// Consistent F5 Tcl 8.4 logical context in which an authored provider may run.
/// The capability supplies no actual C interpreter, object or compiler proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthoredF5Tcl84Core {
    _private: (),
}

impl crate::InvocationDialect {
    /// Validate the logical context for separately authored F5 core providers.
    /// A provider still has to be requested and selected for each semantic axis.
    #[must_use]
    pub fn authored_f5_tcl84_core(self) -> Option<AuthoredF5Tcl84Core> {
        use tcl_dialect::model::Family;
        let family = self.native_family.or(self
            .core_point
            .map(tcl_dialect::model::DialectPoint::family))?;
        if !matches!(family, Family::F5Tcl | Family::F5Irules)
            || self.tcl_version != Some(tcl_dialect::TclVersion::V8_4)
            || self.core_point.is_some_and(|point| {
                point.family() != family
                    || point.tcl_version() != Some(tcl_dialect::TclVersion::V8_4)
            })
        {
            return None;
        }
        Some(AuthoredF5Tcl84Core { _private: () })
    }
}
