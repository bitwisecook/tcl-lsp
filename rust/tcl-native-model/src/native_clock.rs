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

//! Measured `clock` ensemble bootstrap geometry and private worker
//! registrations, independent of handler effects.

use tcl_dialect::TclVersion;

impl crate::InvocationDialect {
    /// Measured C8.6 transition-table worker, independent of its effects.
    #[must_use]
    pub fn clock_get_date_fields_registration(self) -> Option<&'static str> {
        (self.family() == Some(tcl_dialect::model::Family::Tcl)
            && self.tcl_version == Some(TclVersion::V8_6))
        .then_some("::tcl::clock::GetDateFields")
    }

    /// Measured C8.6 process-environment worker constructor.
    #[must_use]
    pub fn clock_getenv_registration(self) -> Option<&'static str> {
        (self.family() == Some(tcl_dialect::model::Family::Tcl)
            && self.tcl_version == Some(TclVersion::V8_6))
        .then_some("::tcl::clock::getenv")
    }

    /// Measured C8.6 private worker constructor, separate from handler effects.
    #[must_use]
    pub fn clock_parse_format_args_registration(self) -> Option<&'static str> {
        (self.family() == Some(tcl_dialect::model::Family::Tcl)
            && self.tcl_version == Some(TclVersion::V8_6))
        .then_some("::tcl::clock::ParseFormatArgs")
    }

    /// Measured clock constructor geometry, independent of handler effects.
    #[must_use]
    pub fn clock_bootstrap_ensemble(self) -> Option<crate::NativeEnsembleBootstrap> {
        if self.family() != Some(tcl_dialect::model::Family::Tcl) {
            return None;
        }
        let deferred_members: &'static [&'static [u8]] = match self.tcl_version? {
            TclVersion::V8_6 => &[b"add", b"format", b"scan"],
            TclVersion::V9_0 | TclVersion::V9_1 => &[],
            _ => return None,
        };
        Some(crate::NativeEnsembleBootstrap {
            namespace: "::tcl::clock",
            deferred_members,
        })
    }

    /// Complete initial public clock map measured on C Tcl 9.0.4.
    #[must_use]
    pub fn clock_complete_stock_ensemble_members(self) -> Option<&'static [&'static str]> {
        (self.family() == Some(tcl_dialect::model::Family::Tcl)
            && self.tcl_version == Some(TclVersion::V9_0))
        .then_some(&[
            "add",
            "clicks",
            "format",
            "microseconds",
            "milliseconds",
            "scan",
            "seconds",
        ])
    }
}
