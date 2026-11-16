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
fn original_instance_descriptor_keeps_registry_identity_prefixes_and_lifecycle() {
    // naming.source.original-registered-instance-words
    // docs/design/analysis/name-resolution-proofs/source-original-registered-instance-words.md
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let registry = crate::registry::CommandRegistry::build_default().project_for_profile(profile);
    let query = Some(profile.surface_query());
    let factory = registry.get_for_surface("ttk::treeview", query).unwrap();
    let method = registry
        .instance_method_for_descriptor_at(factory, "conf", None, query)
        .unwrap();
    assert_eq!(method.name, "configure");
    assert!(std::ptr::eq(
        method,
        registry
            .instance_method_at(factory.name, "conf", None, query)
            .unwrap()
    ));
    let foreign = factory.clone();
    assert!(
        registry
            .instance_methods_for_descriptor_at(&foreign, None, query)
            .is_empty()
    );
    assert!(
        registry
            .resolve_structured_instance_invocation_for_descriptor(
                &foreign,
                crate::InvocationWords::literals(".t", &["configure"]),
                None,
                query,
            )
            .is_none()
    );
    assert!(
        registry
            .resolve_structured_instance_invocation(
                factory.name,
                crate::InvocationWords::literals(".t", &["move", "one"]),
                query,
            )
            .is_none(),
        "the named effect facade still requires independent package admission"
    );
    let packages = [tcl_dialect::model::PackageFloor::at("Tk", "8.6")];
    assert!(
        registry
            .resolve_structured_instance_invocation(
                factory.name,
                crate::InvocationWords::literals(".t", &["move", "one"]),
                query.map(|query| query.with_packages(&packages)),
            )
            .is_some()
    );
    let selected = registry
        .resolve_structured_instance_invocation_for_descriptor(
            factory,
            crate::InvocationWords::literals(".t", &["move", "one"]),
            None,
            query,
        )
        .unwrap();
    assert_eq!(
        selected
            .authored_source_descriptors()
            .subcommand
            .unwrap()
            .name,
        "move"
    );
    assert_eq!(
        selected.facts().arity_accepts_frozen_arguments(),
        Some(false)
    );
}
