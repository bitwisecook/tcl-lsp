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

//! `tcl-vm`'s integration tests that need no source compiler: they drive the
//! VM through its public API, `test_support` constructors and hand-built
//! bytecode. Tests that compile Tcl are in `tcl-vm-it`.

mod expr_surface_e2e;
mod library_bootstrap_presence;
mod native_byte_literals;
mod native_variable_declaration;
mod thread_exit_retirement;

tcl_registry::testing::install_shipped!();
