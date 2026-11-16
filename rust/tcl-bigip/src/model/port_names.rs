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

//! BIG-IP mcpd service-name → port-number table.
//!
//! The canonical mcpd service-name table, filed in four key ranges. Keys are
//! pre-lowercased; lookup is a binary search of each range.

mod d_through_i;
mod j_through_p;
mod q_through_z;
mod through_c;

/// The table in key order: each section is sorted, and every key of one
/// sorts before every key of the next.
static SECTIONS: [&[(&str, i64)]; 4] = [
    through_c::NAMES,
    d_through_i::NAMES,
    j_through_p::NAMES,
    q_through_z::NAMES,
];

/// Return the BIG-IP port number for a (already lowercased) service `name`,
/// or `None` if unknown. (case-folding is the caller's).
#[must_use]
pub fn name_to_port(name: &str) -> Option<i64> {
    SECTIONS.iter().find_map(|section| {
        section
            .binary_search_by(|(k, _)| (*k).cmp(name))
            .ok()
            .map(|i| section[i].1)
    })
}

#[cfg(test)]
mod tests;
