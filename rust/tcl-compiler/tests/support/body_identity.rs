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

use tcl_compiler::ir::ProcedureId;

#[path = "body_identity_lookup.rs"]
mod lookup;

pub use lookup::{FixtureBody, fixture_id_if_present};

// Authored fixtures identify one retained body before accessing its issued ID.
pub fn fixture_id<'a, T: FixtureBody + 'a>(
    inventory: impl IntoIterator<Item = (&'a ProcedureId, &'a T)>,
    display: &str,
) -> ProcedureId {
    fixture_id_if_present(inventory, display)
        .unwrap_or_else(|| panic!("fixture body {display:?} absent"))
}
