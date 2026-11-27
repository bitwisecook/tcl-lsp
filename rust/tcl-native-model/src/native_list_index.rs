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

//! Native list-index argument diagnostics, independent of compiler selection.

impl crate::InvocationDialect {
    /// Actual list-index usage tail for a selected native interpreter.
    #[must_use]
    pub fn list_index_usage(self) -> Option<&'static str> {
        if let Some(version) = self.tcl_version {
            Some(if version < tcl_dialect::TclVersion::V8_6 {
                "lindex list ?index...?"
            } else {
                "lindex list ?index ...?"
            })
        } else {
            self.core_point
                .is_some_and(|point| point.release() == tcl_dialect::model::Release::JIM_0_84)
                .then_some("lindex list ?index ...?")
        }
    }
}
