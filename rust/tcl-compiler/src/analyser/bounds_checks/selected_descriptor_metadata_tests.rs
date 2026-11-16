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
fn catalogue_bounds_selection_keeps_exact_rooted_descriptor_and_missing_geometry() {
    let mut registry = tcl_registry::CommandRegistry::build_default();
    let mut rooted = registry.get_exact("incr").unwrap().clone();
    rooted.name = "::incr";
    rooted.traits = tcl_registry::Traits::empty();
    registry.insert(rooted);
    let owner = BoundsMetadataContext::Standalone(Some(&registry));
    assert_eq!(owner.spec("incr").unwrap().descriptor_key().label(), "incr");
    assert_eq!(
        owner.spec("::incr").unwrap().descriptor_key().label(),
        "::incr"
    );
    for label in [
        ":::::incr",
        "incr::",
        "incr:::member",
        ":incr",
        "missing",
        "",
    ] {
        assert!(owner.spec(label).is_none());
    }
}
