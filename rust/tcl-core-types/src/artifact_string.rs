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

//! Exact NUL-terminated strings and 32-bit string tables for binary artifacts.
//!
//! These types admit artifact representation only. They do not select Tcl name
//! semantics, normalize bytes, or establish any runtime binding.

use alloc::ffi::CString;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;

/// A failure to represent exact input bytes in an artifact string table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactStringError {
    /// An input byte would terminate the artifact entry before its actual end.
    InteriorNul { position: usize },
    /// An offset or terminator-inclusive table length exceeds 32 bits.
    TableTooLarge,
}

impl fmt::Display for ArtifactStringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InteriorNul { position } => {
                write!(f, "artifact string contains NUL at byte {position}")
            }
            Self::TableTooLarge => f.write_str("artifact string table exceeds 32-bit offsets"),
        }
    }
}

impl core::error::Error for ArtifactStringError {}

/// One exact artifact entry with a single final NUL and no interior NUL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactCString(CString);

impl ArtifactCString {
    /// Validate the entire input without truncation or replacement.
    ///
    /// # Errors
    /// Returns [`ArtifactStringError::InteriorNul`] for any NUL in the input.
    pub fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, ArtifactStringError> {
        CString::new(bytes)
            .map(Self)
            .map_err(|error| ArtifactStringError::InteriorNul {
                position: error.nul_position(),
            })
    }

    /// The exact admitted bytes, excluding the terminator.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// The exact admitted bytes and their final terminator.
    #[must_use]
    pub fn as_bytes_with_nul(&self) -> &[u8] {
        self.0.as_bytes_with_nul()
    }
}

/// A NUL-led artifact string table with checked 32-bit offsets and length.
///
/// Entries retain insertion order, including duplicates and empty entries.
#[derive(Debug, Clone)]
pub struct ArtifactStringTable {
    bytes: Vec<u8>,
}

impl Default for ArtifactStringTable {
    fn default() -> Self {
        Self::new()
    }
}

impl ArtifactStringTable {
    /// Create the conventional empty entry at offset zero.
    #[must_use]
    pub fn new() -> Self {
        Self { bytes: vec![0] }
    }

    /// Append exact input bytes, refusing invalid entries before mutation.
    ///
    /// # Errors
    /// Returns [`ArtifactStringError::InteriorNul`] for invalid input or
    /// [`ArtifactStringError::TableTooLarge`] when its offset or final length
    /// cannot be represented in 32 bits.
    pub fn add(&mut self, bytes: impl AsRef<[u8]>) -> Result<u32, ArtifactStringError> {
        let entry = ArtifactCString::new(bytes.as_ref())?;
        let (offset, _) = Self::checked_extent(self.bytes.len(), entry.as_bytes_with_nul().len())?;
        self.bytes.extend_from_slice(entry.as_bytes_with_nul());
        Ok(offset)
    }

    fn checked_extent(
        length: usize,
        entry_length: usize,
    ) -> Result<(u32, u32), ArtifactStringError> {
        let offset = u32::try_from(length).map_err(|_| ArtifactStringError::TableTooLarge)?;
        let end = length
            .checked_add(entry_length)
            .and_then(|end| u32::try_from(end).ok())
            .ok_or(ArtifactStringError::TableTooLarge)?;
        Ok((offset, end))
    }

    /// The complete serialized table, including all terminators.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_entry_preserves_the_table_and_cannot_alias_a_valid_prefix() {
        let mut table = ArtifactStringTable::new();
        assert_eq!(table.add("entry"), Ok(1));
        let before = table.as_bytes().to_vec();
        assert_eq!(
            table.add(b"entry\0suffix"),
            Err(ArtifactStringError::InteriorNul { position: 5 })
        );
        assert_eq!(table.as_bytes(), before);
        assert_eq!(table.add("λ"), Ok(7));
        assert_eq!(table.as_bytes(), b"\0entry\0\xce\xbb\0");
        assert_eq!(table.add(""), Ok(10));
        assert_eq!(table.add("entry"), Ok(11));
    }

    #[test]
    fn terminator_must_fit_the_32_bit_table_extent() {
        let last = u32::MAX as usize;
        assert_eq!(
            ArtifactStringTable::checked_extent(last - 1, 1),
            Ok((u32::MAX - 1, u32::MAX))
        );
        assert_eq!(
            ArtifactStringTable::checked_extent(last, 1),
            Err(ArtifactStringError::TableTooLarge)
        );
        assert_eq!(
            ArtifactStringTable::checked_extent(usize::MAX, 1),
            Err(ArtifactStringError::TableTooLarge)
        );
    }
}
