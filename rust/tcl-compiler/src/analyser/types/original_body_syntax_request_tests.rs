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

#[test]
fn equal_source_and_same_realm_do_not_replace_body_inventory_request() {
    use crate::registry_invocation::source_structure::OriginalBodySyntaxInventory;
    use std::sync::Arc;
    let source = "proc sample {x} {if {$x <= 1} { return 1 }}\nsample 0\n";
    let first = crate::analyser::Analyser::new()
        .analyse(source, "tcl8.6")
        .clone();
    let other = crate::analyser::Analyser::new()
        .analyse(source, "tcl8.6")
        .clone();
    let inventory = OriginalBodySyntaxInventory::capture(source, &first)
        .expect("actual retained original body request");
    let mut changed_request = first.clone();
    changed_request
        .optimiser_source_origin
        .clone_from(&other.optimiser_source_origin);
    assert!(Arc::ptr_eq(
        first.retained_command_realm_arc().unwrap(),
        changed_request.retained_command_realm_arc().unwrap(),
    ));
    assert!(
        !first
            .optimiser_source_origin
            .as_ref()
            .unwrap()
            .same_request_allocation(changed_request.optimiser_source_origin.as_ref().unwrap(),)
    );
    assert!(!inventory.matches_analysis(source, &changed_request));
    assert!(inventory.matches_analysis(source, &first));
}
