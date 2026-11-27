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

//! `Tcl_Init`'s search for `init.tcl`, as each emulated release performs it.
//!
//! Through 9.0 the search is the `tclInit` script `Tcl_Init` evaluates (each
//! release's text, transcribed from its sources, with the candidates tclvm
//! cannot have — zipfs, a configure-time directory — marked and dropped); 9.1
//! moved it into C (`LocatePreInitScript`), transcribed here as Tcl.

use tcl_dialect::TclVersion;

/// The script that locates and sources `init.tcl` for `release`.
pub(crate) const fn search_script(release: TclVersion) -> &'static str {
    match release {
        TclVersion::V8_4 => include_str!("library_init/tcl8.4.tcl"),
        TclVersion::V8_5 => include_str!("library_init/tcl8.5.tcl"),
        TclVersion::V8_6 => include_str!("library_init/tcl8.6.tcl"),
        TclVersion::V9_0 => include_str!("library_init/tcl9.0.tcl"),
        TclVersion::V9_1 => include_str!("library_init/tcl9.1.tcl"),
    }
}
