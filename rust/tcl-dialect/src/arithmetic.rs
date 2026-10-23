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

//! Integer representation and overflow rules of the selected native engine.

/// Integer arithmetic policy, independent of accepted numeral spellings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeArithmetic {
    /// Tcl 8.4's fixed-width wide integers; large shift counts collapse.
    Tcl84Wide,
    /// Tcl 8.5 and later promote overflowing integers to arbitrary precision.
    TclBignum,
    /// Jim's wide integers wrap; shift counts are reduced modulo 64.
    JimWide,
}

impl NativeArithmetic {
    /// Whether a completed expression canonicalises a numeric-looking string.
    /// Jim preserves string operands; its bare numeric literals are converted
    /// separately when the expression evaluates those literals.
    #[must_use]
    pub const fn normalizes_expression_result(self) -> bool {
        !matches!(self, Self::JimWide)
    }

    /// Arithmetic semantics of a C Tcl release.
    #[must_use]
    pub const fn for_tcl_version(version: crate::TclVersion) -> Self {
        match version {
            crate::TclVersion::V8_4 => Self::Tcl84Wide,
            _ => Self::TclBignum,
        }
    }
}
