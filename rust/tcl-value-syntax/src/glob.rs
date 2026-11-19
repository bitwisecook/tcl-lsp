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

//! Shared source roles for native object and name glob patterns.

/// A source construct in an object/name glob. Filesystem globbing has a
/// further path/brace grammar and must select that grammar independently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlobRole {
    AnySequence,
    AnyCharacter,
    CharacterClass,
    Escape,
}

impl GlobRole {
    /// First-unit dispatch shared by the matcher and its readonly source view.
    #[must_use]
    pub const fn from_unit(unit: u32) -> Option<Self> {
        match unit {
            42 => Some(Self::AnySequence),
            63 => Some(Self::AnyCharacter),
            91 => Some(Self::CharacterClass),
            92 => Some(Self::Escape),
            _ => None,
        }
    }
}

/// One complete, subject-independent pattern component in the input's units.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlobComponent {
    pub range: std::ops::Range<usize>,
    pub role: GlobRole,
}
