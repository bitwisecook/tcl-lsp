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

use super::*;

#[test]
fn metadata_string_reader_does_not_turn_unknown_source_profile_into_tcl9() {
    let mut engine = TclVmEngine::new();
    assert!(engine.host_metadata_list_reader().is_some());
    engine
        .vm
        .set_dialect_profile(tcl_dialect::DialectProfile::plain_tcl());
    assert!(engine.host_metadata_list_reader().is_none());
}
