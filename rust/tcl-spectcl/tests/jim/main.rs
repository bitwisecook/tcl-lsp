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

//! `tcl-spectcl`'s tests of Jim's compiled-in surface. Each test installs
//! this crate's surface list ([`tcl_spectcl::core_surfaces::SURFACES`])
//! before its first registry read, so the harness installs nothing at
//! start-up: a shipped-only install would conflict with it.

mod disabled_command_wording;
mod jim_authored_call;
mod jim_core_metadata_identity;
mod jim_document;
mod jim_generation_zero;
mod jim_own_surface;
mod jim_static_read_alternatives;
mod jim_static_source_uncertainty;
mod jim_surface_roster;
mod roster_reload_static_view;

/// Install the surface list these tests read, Jim's among them.
fn install() -> &'static tcl_registry::SurfaceSet {
    tcl_spectcl::core_surfaces::install().expect("the tcl-spectcl surface list installs")
}
