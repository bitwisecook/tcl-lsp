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

/// The three properties this overlay reads are the ones the shipping
/// BIG-IP object schema declares for `sys application template` — the
/// overlay and the schema cannot drift apart silently.
#[test]
fn the_properties_exist_in_the_bigip_object_schema() {
    let registry = tcl_bigip_schema::BigipRegistry::build();
    let spec = registry
        .get("sys_application_template")
        .expect("the template object kind");
    let mut names = std::collections::HashSet::new();
    let mut stack: Vec<&tcl_bigip_schema::BigipPropertySpec> = spec.properties.iter().collect();
    while let Some(property) = stack.pop() {
        names.insert(property.name);
        stack.extend(property.block.iter());
    }
    for property in [
        "requires-bigip-version-min",
        "requires-bigip-version-max",
        "role-acl",
        "run-as",
    ] {
        assert!(names.contains(property), "{property} missing from schema");
    }
}
