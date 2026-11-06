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

//! Native expression command argv composition, separate from expression syntax.

/// How a native expression command receives its source object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExpressionArguments {
    /// C Tcl concatenates one or more source arguments, separated by spaces.
    Concatenate,
    /// Current Jim accepts exactly one original source value object.
    Single,
}

impl ExpressionArguments {
    /// Whether native argv validation reaches the expression source conversion.
    #[must_use]
    pub const fn accepts_len(self, count: usize) -> bool {
        match self {
            Self::Concatenate => count >= 1,
            Self::Single => count == 1,
        }
    }

    /// Native usage text used after argument validation fails.
    #[must_use]
    pub const fn synopsis(self) -> &'static str {
        match self {
            Self::Concatenate => "expr arg ?arg ...?",
            Self::Single => "expr expression",
        }
    }
}
