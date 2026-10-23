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

/// Inclusive native range operands; construction grants no compiler authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeCompiledListRange {
    /// Native first coordinate after the selected compiler's clamping.
    pub first: NativeCompiledListIndex,
    /// Native last coordinate after the selected compiler's clamping.
    pub last: NativeCompiledListIndex,
}

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

    /// Decode a native coordinate against this operation's meaning of `end`.
    /// Range uses the last member; insertion uses the position after it.
    /// Before/after sentinel clamping remains the caller's native recipe.
    #[must_use]
    pub fn decode(self, end: i128) -> i128 {
        if self.0 <= -2 {
            end + i128::from(self.0) + 2
        } else {
            i128::from(self.0)
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_range_and_insertion_decode_against_their_actual_end() {
        let end = NativeCompiledListIndex::from_encoded(-2);
        let before_end = NativeCompiledListIndex::from_encoded(-3);
        assert_eq!(end.decode(2), 2);
        assert_eq!(end.decode(3), 3);
        assert_eq!(before_end.decode(2), 1);
        assert_eq!(NativeCompiledListIndex::from_encoded(-1).decode(2), -1);
        assert_eq!(
            NativeCompiledListIndex::from_encoded(i32::MAX).decode(2),
            i128::from(i32::MAX)
        );
        assert_eq!(end.resolve(0), None);
        assert_eq!(end.resolve(3), Some(2));
    }
}
