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

//! Pure coordinates of C Tcl's compiled list-index operand.

/// Native `TclIndexEncode` coordinate; this is not a runtime index object.
/// Its construction grants no compiler or interpreter authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeCompiledListIndex(i32);

impl NativeCompiledListIndex {
    /// Preserve the native signed operand: `-1` is outside and `-2` is `end`.
    #[must_use]
    pub const fn from_encoded(encoded: i32) -> Self {
        Self(encoded)
    }

    /// Original native instruction operand, independent of portable encodings.
    #[must_use]
    pub const fn encoded(self) -> i32 {
        self.0
    }

    /// Select a member after the original List getter established its length.
    #[must_use]
    pub fn resolve(self, length: usize) -> Option<usize> {
        let position = match self.0 {
            -1 => return None,
            encoded if encoded <= -2 => i128::try_from(length).ok()? - 1 + i128::from(encoded) + 2,
            encoded => i128::from(encoded),
        };
        usize::try_from(position)
            .ok()
            .filter(|index| *index < length)
    }
}
