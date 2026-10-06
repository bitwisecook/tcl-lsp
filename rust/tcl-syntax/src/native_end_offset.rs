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

//! Pure storage and updater law of C Tcl's original end-offset primary.

use tcl_dialect::TclVersion;

/// Original end-offset representation; this data grants no object issuer authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeEndOffset {
    version: TclVersion,
    encoded: i64,
}
impl NativeEndOffset {
    /// Retain the value written by the selected native index conversion.
    #[must_use]
    pub const fn new(version: TclVersion, encoded: i64) -> Self {
        Self { version, encoded }
    }
    /// Physical release owning the representation.
    #[must_use]
    pub const fn version(self) -> TclVersion {
        self.version
    }
    /// Resolve the stored representation against the actual last index.
    #[must_use]
    pub fn resolve(self, end: i64) -> i64 {
        if self.version < TclVersion::V9_0 {
            return i64::from(tcl_syntax_integer_low(self.encoded.wrapping_add(end)));
        }
        match self.encoded {
            i64::MAX => {
                if end == -1 {
                    i64::MAX
                } else {
                    end + 1
                }
            }
            i64::MIN => {
                if end == -1 {
                    i64::MIN
                } else {
                    -1
                }
            }
            offset if offset < 0 => end.saturating_add(offset).saturating_add(1),
            absolute => absolute,
        }
    }
    /// C8 updater bytes; C9's descriptor has no string updater.
    #[must_use]
    pub fn string_update(self) -> Option<Vec<u8>> {
        if self.version >= TclVersion::V9_0 {
            return None;
        }
        Some(if self.encoded == 0 {
            b"end".to_vec()
        } else {
            format!("end-{}", self.encoded.wrapping_neg()).into_bytes()
        })
    }
}
fn tcl_syntax_integer_low(value: i64) -> i32 {
    crate::number::native_int32_low_bits(value)
}
