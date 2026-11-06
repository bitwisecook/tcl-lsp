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

//! Pinned `Tcl_UniCharToLower` unit mapping, independent of a matching purpose.
use tcl_dialect::TclVersion;
#[path = "native_glob_case_mapping.rs"]
mod mapping;

#[must_use]
pub fn lower(version: TclVersion, unit: u32) -> u32 {
    let ranges = match version {
        TclVersion::V8_4 => mapping::C84,
        TclVersion::V8_5 => mapping::C85,
        TclVersion::V8_6 => mapping::C86,
        TclVersion::V9_0 => mapping::C90,
        TclVersion::V9_1 => mapping::C91,
    };
    let index = ranges.partition_point(|&(start, _, _, _)| start <= unit);
    if let Some(&(start, end, stride, delta)) =
        index.checked_sub(1).and_then(|index| ranges.get(index))
        && unit <= end
        && (unit - start).is_multiple_of(stride)
    {
        return unit
            .checked_add_signed(delta)
            .expect("pinned Tcl lowercase unit range");
    }
    unit
}
