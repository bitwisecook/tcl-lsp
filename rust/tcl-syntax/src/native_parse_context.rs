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

//! Native parse-error source extents, independent of execution permission.

/// C8.4 compiler context from the original remaining command and parser term.
/// A failed `Tcl_ParseCommand` spans to the supplied source end. Compilation
/// removes the terminator only when it is the final byte of that span.
/// The caller must supply independently proved parser geometry; this function
/// neither parses source nor attests a native parse failure.
#[must_use]
pub fn c84_compilation_command_extent(
    source: &[u8],
    command_start: usize,
    term: usize,
) -> Option<&[u8]> {
    if command_start > term || term >= source.len() {
        return None;
    }
    let end = if term == source.len() - 1 {
        term
    } else {
        source.len()
    };
    source.get(command_start..end)
}
