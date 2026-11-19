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

//! Source-expression and native-getter adapters over the shared numeric grammar.

pub use tcl_numeric::number::*;

/// Validate the original C8.4 decimal unsigned-long stage with the checked
/// 8/4/8 target. The actual libc caller owns errno and conversion; this kernel
/// only selects full C-string consumption, native int narrowing and bounds.
#[must_use]
pub fn c84_double_precision_from_unsigned_long(
    original: &[u8],
    value: u64,
    end: usize,
    _target: crate::scalar_getter::NativeScalarGetterTarget,
) -> Option<u8> {
    let original = tcl_core_types::c_string_extent(original);
    if end == 0 || end != original.len() {
        return None;
    }
    let signed = i64::from_ne_bytes(value.to_ne_bytes());
    double_precision_from_native_integer(
        tcl_dialect::DoubleStringPolicy::Tcl84Precision,
        i64::from(native_int32_low_bits(signed)),
    )
}
