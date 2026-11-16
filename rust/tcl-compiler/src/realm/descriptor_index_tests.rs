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
fn realm_descriptor_index_keeps_exact_coordinates_and_declines_collisions() {
    let mut registry = CommandRegistry::build_default();
    for label in [
        "NAME_REALM::",
        "NAME_REALM:::member",
        "::::NAME_REALM",
        "NAME_REALM_COLLISION",
        "::NAME_REALM_COLLISION",
    ] {
        registry.insert(tcl_registry::CommandSpec {
            name: label,
            ..tcl_registry::CommandSpec::DEFAULT
        });
    }
    let realm =
        document_realm_bindings_with_config("", tcl_lexer::LexerConfig::default(), &registry);
    for coordinate in ["::NAME_REALM::", "::NAME_REALM:::member", "::::NAME_REALM"] {
        assert!(realm.specs.contains_key(coordinate), "{coordinate}");
    }
    for missing in [
        "::NAME_REALM",
        "::NAME_REALM::member",
        "::NAME_REALM_COLLISION",
    ] {
        assert!(!realm.specs.contains_key(missing), "{missing}");
    }
}
